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

use cached::proc_macro::once;
#[cfg(feature = "reqwest")]
use once_cell::sync::{Lazy, OnceCell};
#[cfg(feature = "reqwest")]
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
#[cfg(feature = "reqwest")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "reqwest")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "reqwest")]
use std::thread;
#[cfg(feature = "reqwest")]
use std::time::Duration;

/// Timeout for API requests in seconds.
#[cfg(feature = "reqwest")]
const API_REQUEST_TIMEOUT_SECS: u64 = 120;

/// Creates a new reqwest HTTP client with a default timeout.
#[cfg(feature = "reqwest")]
#[once(result = true, sync_writes = true)]
fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(API_REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")
}

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
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = http_client()?;
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

// ---------------- Lock renewal thread (select/guess lifecycle) ----------------

/// The duration for which a lock is valid.
#[cfg(feature = "reqwest")]
const LOCK_TTL: Duration = Duration::from_secs(30);
/// The interval at which the lock is renewed.
#[cfg(feature = "reqwest")]
const LOCK_RENEW_INTERVAL: Duration = Duration::from_secs(5);

/// Manages the state of the background lock renewal thread.
#[cfg(feature = "reqwest")]
struct LockRunner {
    /// An atomic boolean to signal the thread to stop.
    stop: Arc<AtomicBool>,
    /// The handle to the renewal thread.
    handle: Option<std::thread::JoinHandle<()>>,
    /// The lock token.
    token: String,
}

/// A global, mutex-protected `Option<LockRunner>` to manage the lock renewal process.
#[cfg(feature = "reqwest")]
static LOCK_MANAGER: Lazy<Mutex<Option<LockRunner>>> = Lazy::new(|| Mutex::new(None));
/// A flag to ensure the Ctrl+C handler is installed only once.
#[cfg(feature = "reqwest")]
static CTRL_C_INSTALLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Starts the lock manager, acquiring a lock and spawning a renewal thread.
///
/// This function is idempotent. If the lock manager is already running, it does nothing.
/// Otherwise, it acquires a lock with retries and then spawns a background thread
/// that periodically extends the lock's TTL. It also installs a Ctrl+C handler
/// to attempt a best-effort unlock on interrupt.
#[cfg(feature = "reqwest")]
fn start_lock_manager_blocking() -> Result<()> {
    // Already running? nothing to do.
    if LOCK_MANAGER
        .lock()
        .expect("LOCK_MANAGER mutex was poisoned")
        .is_some()
    {
        return Ok(());
    }

    // Acquire lock with retries every 5 seconds.
    eprintln!("Acquiring lock...");
    let token = loop {
        match crate::lock::lock(LOCK_TTL)? {
            Some(t) => {
                eprintln!("Lock acquired.");
                break t;
            }
            None => {
                eprintln!("Failed to acquire lock, retrying in 5s...");
                thread::sleep(LOCK_RENEW_INTERVAL)
            }
        }
    };

    let stop = Arc::new(AtomicBool::new(false));
    // Install a Ctrl+C handler once; it will attempt a best-effort unlock.
    if !CTRL_C_INSTALLED.swap(true, Ordering::SeqCst) {
        let stop_for_sig = stop.clone();
        let token_for_sig = token.clone();
        let _ = ctrlc::set_handler(move || {
            eprintln!("Ctrl+C detected, unlocking and exiting.");
            let _ = crate::lock::unlock(&token_for_sig, false);
            stop_for_sig.store(true, Ordering::SeqCst);
            // Exit immediately after unlocking on Ctrl+C, following standard signal behavior.
            std::process::exit(130);
        });
    }

    let token_clone = token.clone();
    let stop_clone = stop.clone();
    // Spawn the renewal thread.
    let handle = thread::spawn(move || {
        let mut consecutive_failures = 0u32;
        loop {
            // Sleep in 100ms ticks to respond quickly to stop requests, for a total of 5s per cycle.
            for _ in 0..50 {
                if stop_clone.load(Ordering::SeqCst) {
                    // The lock manager was stopped, perform cleanup and exit thread.
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }

            // Attempt to extend the lock.
            match crate::lock::extend(&token_clone, LOCK_TTL) {
                Ok(true) => {
                    // Success: reset failure streak.
                    consecutive_failures = 0;
                }
                Ok(false) => {
                    // Extension was explicitly rejected (e.g., token mismatch or expired):
                    // this indicates the lock cannot be continued; exit the process immediately.
                    eprintln!("Lock extend rejected; exiting immediately.");
                    std::process::exit(1);
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!(
                        "Lock extend error (streak {} / 6): {}",
                        consecutive_failures, e
                    );
                }
            }
            // If lock extension fails too many times, assume a persistent issue and exit.
            if consecutive_failures >= 6 {
                eprintln!("Lock extend failed 6 times consecutively; exiting process.");
                std::process::exit(1);
            }
        }
    });

    // Store the runner in the global manager.
    *LOCK_MANAGER.lock().unwrap() = Some(LockRunner {
        stop,
        handle: Some(handle),
        token,
    });
    Ok(())
}

/// Stops the lock manager, signals the renewal thread to exit, and unlocks.
///
/// This function is idempotent. It signals the background thread to stop,
/// waits for it to join, and then explicitly unlocks.
#[cfg(feature = "reqwest")]
fn stop_lock_manager_blocking() {
    let mut mgr = LOCK_MANAGER.lock().unwrap();
    if let Some(mut lr) = mgr.take() {
        eprintln!("Stopping lock manager and unlocking...");
        lr.stop.store(true, Ordering::SeqCst);
        if let Some(h) = lr.handle.take()
            && let Err(e) = h.join()
        {
            eprintln!("Lock renewal thread panicked: {:?}", e);
        }
        // Final unlock attempt.
        let _ = crate::lock::unlock(&lr.token, false);
        eprintln!("Unlock complete.");
    }
}

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
///
/// # Returns
///
/// The `problemName` echoed by the service on success.
#[cfg(feature = "reqwest")]
pub fn select(problem_name: &str) -> Result<String> {
    // Acquire process-wide lock and start renewal thread.
    start_lock_manager_blocking()?;
    let client = http_client()?;
    let url = format!("{}/select", aedificium_base());

    // Obtain id via get_id (parsed from id.json).
    let id = get_id()?;
    let req = SelectRequest {
        id: id.as_str(),
        problem_name,
    };
    let res = client
        .post(url)
        .json(&req)
        .send()
        .context("Failed to POST /select")?;
    let status = res.status();
    log_unagi_header(&res);
    if !status.is_success() {
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/select returned {}: {}", status, body);
    }

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
    let client = http_client()?;
    let url = format!("{}/explore", aedificium_base());
    let id = get_id()?;
    // Convert the plans from Vec<usize> to strings of digits for the JSON request.
    let plans_vec: Vec<String> = plans.into_iter().map(|s| s.as_ref().to_string()).collect();
    let req = ExploreRequest {
        id: id.as_str(),
        plans: &plans_vec,
    };

    let res = client
        .post(url)
        .json(&req)
        .send()
        .context("Failed to POST /explore")?;
    let status = res.status();
    log_unagi_header(&res);
    if !status.is_success() {
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/explore returned {}: {}", status, body);
    }

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
    let client = http_client()?;
    let url = format!("{}/guess", aedificium_base());

    let id = get_id()?;
    let req = GuessRequest {
        id: id.as_str(),
        map,
    };

    let res = client
        .post(url)
        .json(&req)
        .send()
        .context("Failed to POST /guess")?;
    let status = res.status();
    log_unagi_header(&res);
    if !status.is_success() {
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/guess returned {}: {}", status, body);
    }

    let body: GuessResponse = res.json().context("Failed to parse /guess response")?;
    // Stop renewal and unlock immediately after a guess is made.
    stop_lock_manager_blocking();
    Ok(body.correct)
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
