use anyhow::{Context, Result};

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

#[cfg(feature = "reqwest")]
const API_REQUEST_TIMEOUT_SECS: u64 = 120;

#[cfg(feature = "reqwest")]
fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(API_REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")
}

/// Fetches `id.json` from the same directory as `bearer.txt`.
///
/// The path is: `https://storage.googleapis.com/icfpc2025-data/{UNAGI_PASSWORD}/id.json`.
// Fetches raw JSON bytes of id.json (blocking)
#[cfg(feature = "reqwest")]
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

// Parses id.json and returns the `id` string field.
#[cfg(feature = "reqwest")]
#[derive(serde::Deserialize)]
struct IdJsonOwned {
    id: String,
}

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

#[cfg(feature = "reqwest")]
fn aedificium_base() -> String {
    std::env::var("AEDIFICIUM_ENDPOINT")
        .ok()
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://icfpc.sx9.jp/api".to_string())
}

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
#[cfg(feature = "reqwest")]
const LOCK_TTL: Duration = Duration::from_secs(30);
#[cfg(feature = "reqwest")]
const LOCK_RENEW_INTERVAL: Duration = Duration::from_secs(5);

#[cfg(feature = "reqwest")]
struct LockRunner {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    token: String,
}

#[cfg(feature = "reqwest")]
static LOCK_MANAGER: Lazy<Mutex<Option<LockRunner>>> = Lazy::new(|| Mutex::new(None));
#[cfg(feature = "reqwest")]
static CTRL_C_INSTALLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[cfg(feature = "reqwest")]
fn start_lock_manager_blocking() -> Result<()> {
    // Already running? nothing to do.
    if LOCK_MANAGER.lock().unwrap().is_some() {
        return Ok(());
    }

    // Acquire lock with retries every 5 seconds.
    let token = loop {
        match crate::lock::lock(LOCK_TTL)? {
            Some(t) => break t,
            None => thread::sleep(LOCK_RENEW_INTERVAL),
        }
    };

    let stop = Arc::new(AtomicBool::new(false));
    // Install Ctrl+C handler once; it will attempt best-effort unlock.
    if !CTRL_C_INSTALLED.swap(true, Ordering::SeqCst) {
        let stop_for_sig = stop.clone();
        let token_for_sig = token.clone();
        let _ = ctrlc::set_handler(move || {
            let _ = crate::lock::unlock(&token_for_sig, false);
            stop_for_sig.store(true, Ordering::SeqCst);
        });
    }

    let token_clone = token.clone();
    let stop_clone = stop.clone();
    let handle = thread::spawn(move || {
        loop {
            for _ in 0..5 {
                if stop_clone.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(Duration::from_millis(1000));
            }
            if stop_clone.load(Ordering::SeqCst) {
                return;
            }
            match crate::lock::extend(&token_clone, LOCK_TTL) {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!("Lock extend rejected; exiting.");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Lock extend error: {}; exiting.", e);
                    std::process::exit(1);
                }
            }
        }
    });

    *LOCK_MANAGER.lock().unwrap() = Some(LockRunner {
        stop,
        handle: Some(handle),
        token,
    });
    Ok(())
}

#[cfg(feature = "reqwest")]
fn stop_lock_manager_blocking() {
    let mut mgr = LOCK_MANAGER.lock().unwrap();
    if let Some(mut lr) = mgr.take() {
        lr.stop.store(true, Ordering::SeqCst);
        if let Some(h) = lr.handle.take() {
            let _ = h.join();
        }
        let _ = crate::lock::unlock(&lr.token, false);
    }
}

#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct SelectRequest<'a> {
    #[serde(rename = "id")]
    id: &'a str,
    #[serde(rename = "problemName")]
    problem_name: &'a str,
}

#[cfg(feature = "reqwest")]
#[derive(Deserialize)]
struct SelectResponse {
    #[serde(rename = "problemName")]
    problem_name: String,
}

/// POST /select to choose a problem to solve.
/// Returns the `problemName` echoed by the service.
#[cfg(feature = "reqwest")]
pub fn select(problem_name: &str) -> Result<String> {
    // Acquire process-wide lock and start renewal thread
    start_lock_manager_blocking()?;
    let client = http_client()?;
    let url = format!("{}/select", aedificium_base());

    // Obtain id via get_id (parsed from id.json)
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

#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct ExploreRequest<'a> {
    id: &'a str,
    plans: &'a [String],
}

#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Deserialize)]
pub struct ExploreResponse {
    pub results: Vec<Vec<usize>>,
    #[serde(rename = "queryCount")]
    pub query_count: u64,
}

/// POST /explore with one or more route plans. Fetches `id` internally.
#[cfg(feature = "reqwest")]
pub fn explore<I, S>(plans: I) -> Result<ExploreResponse>
where
    I: IntoIterator<Item = S>,
    S: AsRef<Vec<usize>>,
{
    let client = http_client()?;
    let url = format!("{}/explore", aedificium_base());
    let id = get_id()?;
    let plans_vec: Vec<String> = plans
        .into_iter()
        .map(|s| {
            s.as_ref()
                .iter()
                .map(|&x| (b'0' + (x as u8)) as char)
                .collect()
        })
        .collect();
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

#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConnectionEnd {
    pub room: usize,
    pub door: usize,
}

#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConnection {
    pub from: MapConnectionEnd,
    pub to: MapConnectionEnd,
}

#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub rooms: Vec<usize>,
    #[serde(rename = "startingRoom")]
    pub starting_room: usize,
    pub connections: Vec<MapConnection>,
}

#[cfg(feature = "reqwest")]
#[derive(Serialize)]
struct GuessRequest<'a> {
    id: &'a str,
    map: &'a Map,
}

#[cfg(feature = "reqwest")]
#[derive(Deserialize)]
struct GuessResponse {
    correct: bool,
}

/// POST /guess to submit a candidate map. Returns whether it is correct.
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
    // Stop renewal and unlock immediately after guess
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
