use actix_web::{HttpResponse, Responder};
use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tokio::task::JoinSet;

#[derive(Debug, Deserialize)]
struct ProblemEntry {
    #[serde(rename = "problem")]
    problem: String,
    #[serde(rename = "size")]
    _size: usize,
}

fn base_endpoint() -> String {
    std::env::var("AEDIFICIUM_ENDPOINT")
        .ok()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com".to_string())
}

async fn run_impl() -> Result<serde_json::Value> {
    let client = Client::new();
    let base = base_endpoint();

    let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let bucket = "icfpc2025-data";
    let prefix = format!("history/{}/", ts);

    // Fetch problem list
    let probs: Vec<ProblemEntry> = client
        .get(format!("{}/select", base))
        .send()
        .await
        .context("Failed to GET /select for problem list")?
        .json()
        .await
        .context("Failed to parse problem list JSON")?;

    // For each problem, fetch leaderboard and store (in parallel)
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
    // Also fetch global leaderboard in parallel
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
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(obj)) => saved.push(obj),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("Join error: {}", e)),
        }
    }

    // Note: global leaderboard handled by the JoinSet above

    Ok(serde_json::json!({
        "timestamp": ts,
        "saved": saved,
    }))
}

pub async fn run() -> impl Responder {
    match run_impl().await {
        Ok(v) => HttpResponse::Ok()
            .content_type("application/json")
            .body(v.to_string()),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
