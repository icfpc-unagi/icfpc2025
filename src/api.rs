//! # Unagi API Client
//!
//! This module provides a client for the Aedificium (Unagi) web service,
//! which is used for the ICFP 2025 contest. It handles authentication,
//! problem selection, exploration, and final map submission (guessing).
//!
//! The client manages a process-wide lock that is automatically acquired
//! when a problem is selected with `select()` and released when a guess is
//! made with `guess()`. A background thread handles lock renewal.
//!
//! All functions in this module are blocking and require the `reqwest` feature.

use anyhow::{Context, Result};

#[cfg(feature = "reqwest")]
use cached::proc_macro::cached;
use cached::proc_macro::once;
#[cfg(feature = "reqwest")]
use once_cell::sync::OnceCell;
#[cfg(feature = "reqwest")]
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::collections::HashMap;
#[cfg(feature = "reqwest")]
use std::time::Duration;
#[cfg(feature = "reqwest")]
use std::time::Instant;

use crate::client;

/// Fetches `id.json` from the contest's Google Cloud Storage bucket.
///
/// The path is constructed using the `UNAGI_PASSWORD` environment variable:
/// `https://storage.googleapis.com/icfpc2025-data/{UNAGI_PASSWORD}/id.json`.
///
/// This function performs a blocking HTTP GET request and returns the raw
/// response body as bytes.
#[cfg(feature = "reqwest")]
#[once(result = true, sync_writes = true)]
pub fn get_id_json() -> anyhow::Result<Vec<u8>> {
    use crate::client;

    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = &*client::BLOCKING_CLIENT;
    let res = client
        .get(format!(
            "https://storage.googleapis.com/icfpc2025-data/{}/id.json",
            unagi_password
        ))
        .send()
        .context("Failed to get id.json")?;
    res.bytes()
        .map(|b| b.to_vec())
        .context("Failed to read id.json body")
}

/// A struct to deserialize the `id` field from `id.json`.
#[cfg(feature = "reqwest")]
#[derive(serde::Deserialize)]
struct IdJsonOwned {
    id: String,
}

/// Fetches, parses, and caches the team `id` from `id.json`.
///
/// On the first call, this function fetches `id.json` using `get_id_json()`,
/// parses it, and caches the `id` value in a static `OnceCell`. Subsequent
/// calls will return the cached value directly without making a network request.
#[cfg(feature = "reqwest")]
pub fn get_id() -> anyhow::Result<String> {
    // Fast path: return cached value if available.
    static ID_CACHE: OnceCell<String> = OnceCell::new();
    if let Some(id) = ID_CACHE.get() {
        return Ok(id.clone());
    }

    // Slow path: fetch and cache.
    let bytes = get_id_json()?;
    let parsed: IdJsonOwned = serde_json::from_slice(&bytes).context("Failed to parse id.json")?;
    let id = parsed.id;
    let _ = ID_CACHE.set(id.clone());
    Ok(id)
}

/// Returns the base URL for the Aedificium API.
///
/// It uses the `AEDIFICIUM_ENDPOINT` environment variable if set, otherwise
/// defaults to `https://icfpc.sx9.jp/api`.
#[cfg(feature = "reqwest")]
#[once]
fn aedificium_base() -> String {
    std::env::var("AEDIFICIUM_ENDPOINT")
        .ok()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://icfpc.sx9.jp/api".to_string())
}

/// Logs the value of the `x-unagi-log` header if present in the response.
#[cfg(feature = "reqwest")]
fn log_unagi_header(res: &reqwest::blocking::Response) {
    let name = reqwest::header::HeaderName::from_static("x-unagi-log");
    if let Some(val) = res.headers().get(name)
        && let Ok(s) = val.to_str()
    {
        eprintln!("X-Unagi-Log: {}", s);
    }
}

/// Returns a retry window for the given HTTP status if it should be retried.
///
/// - 5xx: retry up to 30 minutes
/// - 4xx: retry up to 1 minute
/// - otherwise: not retryable
#[cfg(feature = "reqwest")]
fn retry_window_for_status(status: reqwest::StatusCode) -> Option<Duration> {
    if status.is_server_error() {
        Some(Duration::from_secs(30 * 60))
    } else if status.is_client_error() {
        Some(Duration::from_secs(60))
    } else {
        None
    }
}

/// Performs a POST with JSON body and retries on transient failures.
///
/// Backoff waits 1, 2, 4, ..., up to 32 seconds between attempts, then keeps
/// retrying every 32 seconds until 30 minutes have elapsed since the first
/// attempt. If it still hasn't succeeded by then, the function panics.
#[cfg(feature = "reqwest")]
fn post_json_with_retry<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    body: &T,
    context: &str,
) -> Result<reqwest::blocking::Response> {
    let start = Instant::now();
    let network_deadline = Duration::from_secs(30 * 60);
    let mut delay = Duration::from_secs(1);
    loop {
        match client.post(url).json(body).send() {
            Ok(res) => {
                let status = res.status();
                log_unagi_header(&res);
                if status.is_success() {
                    return Ok(res);
                }
                if let Some(limit) = retry_window_for_status(status) {
                    if start.elapsed() >= limit {
                        panic!("{} failed for over {:?} — aborting", context, limit);
                    }
                    // transient error: fallthrough to sleep and retry
                } else {
                    let body = res.text().unwrap_or_default();
                    anyhow::bail!("{} returned {}: {}", context, status, body);
                }
            }
            Err(err) => {
                // Network/timeout errors: retry until deadline
                eprintln!("{} request error: {} — will retry", context, err);
                if start.elapsed() >= network_deadline {
                    panic!("{} failed for over 30 minutes — aborting", context);
                }
            }
        }
        std::thread::sleep(delay);
        if delay < Duration::from_secs(32) {
            delay = std::cmp::min(delay.saturating_mul(2), Duration::from_secs(32));
        }
    }
}

// ---------------- Lock renewal thread (select/guess lifecycle) ----------------

#[cfg(feature = "reqwest")]
use crate::lock_guard::{start_lock_manager_blocking, stop_lock_manager_blocking};

/// Represents the JSON request body for the `/select` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct SelectRequest<'a> {
    /// The team ID.
    #[serde(rename = "id")]
    id: &'a str,
    /// The name of the problem to solve.
    #[serde(rename = "problemName")]
    problem_name: &'a str,
}

/// Represents the JSON response body from the `/select` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Deserialize)]
struct SelectResponse {
    /// The name of the problem, echoed back by the server.
    #[serde(rename = "problemName")]
    problem_name: String,
}

/// Selects a problem to solve via `POST /select`.
///
/// This action acquires a process-wide lock and starts a background thread
/// to maintain it. The lock is held until `guess()` is called.
///
/// # Arguments
///
/// * `problem_name` - The name of the problem to select.
// lock manager API described in lock_guard
/// # Returns
///
/// The `problemName` echoed by the service on success.
#[cfg(feature = "reqwest")]
pub fn select(problem_name: &str) -> Result<String> {
    // Acquire process-wide lock and start renewal thread.
    start_lock_manager_blocking()?;
    let client = &*client::BLOCKING_CLIENT;
    let url = format!("{}/select", aedificium_base());

    // Obtain id via get_id (parsed from id.json).
    let id = get_id()?;
    let req = SelectRequest {
        id: id.as_str(),
        problem_name,
    };
    let res = post_json_with_retry(client, &url, &req, "/select")?;

    let body: SelectResponse = res.json().context("Failed to parse /select response")?;
    Ok(body.problem_name)
}

/// Represents the JSON request body for the `/explore` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct ExploreRequest<'a> {
    /// The team ID.
    id: &'a str,
    /// A list of exploration plans, where each plan is a string of digits
    /// representing door choices.
    plans: &'a [String],
}

/// Represents the JSON response from the `/explore` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Deserialize)]
pub struct ExploreResponse {
    /// A list of results, where each result corresponds to a plan and contains
    /// a vector of room signatures including steps after rewrite-label actions.
    pub results: Vec<Vec<usize>>,
    /// The total number of queries consumed by this request.
    #[serde(rename = "queryCount")]
    pub query_count: u64,
}

/// Submits one or more route plans for exploration via `POST /explore`.
///
/// This function fetches the team `id` internally.
///
/// # Arguments
///
/// * `plans` - An iterator of exploration plans. Each plan is a slice of `usize`
///   representing a sequence of door choices.
///
/// # Returns
///
/// An `ExploreResponse` containing the results of the exploration.
#[cfg(feature = "reqwest")]
pub fn explore<I, S>(plans: I) -> Result<ExploreResponse>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let client = &*client::BLOCKING_CLIENT;
    let url = format!("{}/explore", aedificium_base());
    let id = get_id()?;
    // Convert the plans from Vec<usize> to strings of digits for the JSON request.
    let plans_vec: Vec<String> = plans.into_iter().map(|s| s.as_ref().to_string()).collect();
    let req = ExploreRequest {
        id: id.as_str(),
        plans: &plans_vec,
    };

    let res = post_json_with_retry(client, &url, &req, "/explore")?;

    let body: ExploreResponse = res.json().context("Failed to parse /explore response")?;
    Ok(body)
}

/// Represents one end of a passage, specified by a room and a door index.
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConnectionEnd {
    /// The index of the room.
    pub room: usize,
    /// The index of the door within that room (0-5).
    pub door: usize,
}

/// Represents a passage between two doors in two rooms.
/// The API documentation refers to this as a "connection".
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConnection {
    /// The "from" side of the passage.
    pub from: MapConnectionEnd,
    /// The "to" side of the passage.
    pub to: MapConnectionEnd,
}

/// Represents the final map structure of the Aedificium to be submitted.
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    /// A list of room signatures. `rooms[i]` is the signature of room `i`.
    pub rooms: Vec<usize>,
    /// The index of the starting room.
    #[serde(rename = "startingRoom")]
    pub starting_room: usize,
    /// A list of passages (connections) between doors.
    pub connections: Vec<MapConnection>,
}

/// Represents the JSON request body for the `/guess` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct GuessRequest<'a> {
    /// The team ID.
    id: &'a str,
    /// The proposed map of the Aedificium.
    map: &'a Map,
}

/// Represents the JSON response from the `/guess` endpoint.
#[cfg(feature = "reqwest")]
#[derive(Deserialize)]
struct GuessResponse {
    /// Whether the submitted map was correct.
    correct: bool,
}

/// Submits a candidate map via `POST /guess` and releases the lock.
///
/// This function fetches the team `id` internally. After the guess is submitted,
/// it stops the lock renewal thread and releases the lock.
///
/// # Arguments
///
/// * `map` - The candidate map to submit.
///
/// # Returns
///
/// `true` if the map was correct, `false` otherwise.
#[cfg(feature = "reqwest")]
pub fn guess(map: &Map) -> Result<bool> {
    let client = &*client::BLOCKING_CLIENT;
    let url = format!("{}/guess", aedificium_base());

    let id = get_id()?;
    let req = GuessRequest {
        id: id.as_str(),
        map,
    };

    let res = post_json_with_retry(client, &url, &req, "/guess")?;

    let body: GuessResponse = res.json().context("Failed to parse /guess response")?;
    // Stop renewal and unlock immediately after a guess is made.
    stop_lock_manager_blocking();
    Ok(body.correct)
}

#[cfg(feature = "reqwest")]
#[cached(result = true, time = 300)]
pub fn scores() -> Result<HashMap<String, i64>> {
    let client = &*client::BLOCKING_CLIENT;
    // This endpoint is not proxied.
    let url = "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com/";

    let id = get_id()?;
    let res = client
        .get(url)
        .query(&[("id", &id)])
        .send()
        .context("Failed to GET scores")?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/ (scores) returned {}: {}", status, body);
    }

    let body = res.json().context("Failed to parse scores response")?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    // Runs only when explicitly enabled (e.g., `make test/unagi`).
    // Requires `UNAGI_PASSWORD` to be set to access the remote object.
    #[ignore]
    #[test]
    fn sha1_of_id_json_matches_expected() -> Result<()> {
        // If UNAGI_PASSWORD isn't set, skip gracefully.
        if std::env::var("UNAGI_PASSWORD").is_err() {
            eprintln!("UNAGI_PASSWORD not set; skipping sha1 check for id.json");
            return Ok(());
        }

        let bytes = get_id_json()?;

        use sha1::{Digest, Sha1};
        let digest = Sha1::digest(&bytes);
        let hex = hex::encode(digest);

        assert_eq!(
            hex, "010bb94e10b85fb5844b2701f2cc93a13c8ba249",
            "SHA1 mismatch for id.json"
        );

        Ok(())
    }
}
