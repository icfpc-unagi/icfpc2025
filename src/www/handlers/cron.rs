//! # Cron Job Handlers
//!
//! This module contains handlers designed to be called periodically by a cron
//! job or a similar scheduling service.

use crate::client;
use actix_web::{HttpResponse, Responder};

use anyhow::{Context, Result};
use chrono::Utc;

use serde::Deserialize;
use tokio::task::JoinSet;

/// A struct to deserialize entries from the problem list endpoint.
#[derive(Debug, Deserialize)]
struct ProblemEntry {
    #[serde(rename = "problem")]
    problem: String,
    #[serde(rename = "size")]
    _size: usize,
}

/// Determines the base endpoint for the Aedificium API.
///
/// Uses the `AEDIFICIUM_ENDPOINT` environment variable if set, otherwise
/// defaults to the official contest server URL.
fn base_endpoint() -> String {
    std::env::var("AEDIFICIUM_ENDPOINT")
        .ok()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com".to_string())
}

/// The core implementation of the leaderboard archiving cron job.
///
/// This function performs the following steps:
/// 1. Fetches the list of all available problems from the `/select` endpoint.
/// 2. Creates a timestamped "directory" path in GCS (e.g., `history/20250906-123000/`).
/// 3. Spawns parallel tasks to fetch the leaderboard JSON for each problem.
/// 4. In parallel, also fetches the global leaderboard.
/// 5. Each task, upon receiving leaderboard data, uploads it as a JSON file to the
///    timestamped path in the `icfpc2025-data` GCS bucket.
/// 6. Waits for all tasks to complete and collects the paths of the saved objects.
///
/// # Returns
/// A `Result` containing a JSON value with the timestamp and a list of all
/// GCS objects that were successfully created.
async fn run_impl() -> Result<serde_json::Value> {
    let client = &*client::CLIENT;
    let base = base_endpoint();

    let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let bucket = "icfpc2025-data";
    let prefix = format!("history/{}/", ts);

    // 1. Fetch problem list.
    let probs: Vec<ProblemEntry> = client
        .get(format!("{}/select", base))
        .send()
        .await
        .context("Failed to GET /select for problem list")?
        .json()
        .await
        .context("Failed to parse problem list JSON")?;

    // 3. For each problem, fetch and store its leaderboard in parallel.
    let mut saved = Vec::new();
    let mut set: JoinSet<Result<String>> = JoinSet::new();
    for p in probs {
        let client = client.clone();
        let base = base.clone();
        let prefix = prefix.clone();
        let bucket = bucket.to_string();
        let problem = p.problem;
        set.spawn(async move {
            let url = format!("{}/leaderboard/{}", base, problem);
            let body = client
                .get(&url)
                .send()
                .await
                .with_context(|| format!("Failed to GET leaderboard for {}", &problem))?
                .text()
                .await
                .with_context(|| format!("Failed to read leaderboard body for {}", &problem))?;

            let object = format!("{}{}.json", prefix, problem);
            crate::gcp::gcs::upload_object(&bucket, &object, body.as_bytes(), "application/json")
                .await
                .with_context(|| format!("Failed to upload {}", object))?;
            Ok(object)
        });
    }

    // 4. Also fetch the global leaderboard in parallel.
    {
        let client = client.clone();
        let base = base.clone();
        let prefix = prefix.clone();
        let bucket = bucket.to_string();
        set.spawn(async move {
            let body = client
                .get(format!("{}/leaderboard/global", base))
                .send()
                .await
                .context("Failed to GET leaderboard/global")?
                .text()
                .await
                .context("Failed to read leaderboard/global body")?;
            let object = format!("{}global.json", prefix);
            crate::gcp::gcs::upload_object(&bucket, &object, body.as_bytes(), "application/json")
                .await
                .context("Failed to upload global.json")?;
            Ok(object)
        });
    }

    // 6. Wait for all archiving tasks to complete.
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(obj)) => saved.push(obj),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("Join error: {}", e)),
        }
    }

    Ok(serde_json::json!({
        "timestamp": ts,
        "saved": saved,
    }))
}

/// The web handler for the `/cron/run` endpoint.
///
/// This function wraps `run_impl`, converting its `Result` into an
/// appropriate `HttpResponse` (Ok or InternalServerError).
pub async fn run() -> impl Responder {
    match run_impl().await {
        Ok(v) => HttpResponse::Ok()
            .content_type("application/json")
            .body(v.to_string()),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
