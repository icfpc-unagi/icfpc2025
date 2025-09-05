use anyhow::{Context, Result};

#[cfg(feature = "reqwest")]
use once_cell::sync::OnceCell;
#[cfg(feature = "reqwest")]
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Fetches `id.json` from the same directory as `bearer.txt`.
///
/// The path is: `https://storage.googleapis.com/icfpc2025-data/{UNAGI_PASSWORD}/id.json`.
// Fetches raw JSON bytes of id.json (blocking)
#[cfg(feature = "reqwest")]
pub fn get_id_json() -> anyhow::Result<Vec<u8>> {
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = Client::new();
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
const AEDIFICIUM_BASE_URL: &str = "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com";

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
    let client = Client::new();
    let url = format!("{}/select", AEDIFICIUM_BASE_URL);

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

    if !res.status().is_success() {
        let status = res.status();
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
    pub results: Vec<Vec<i32>>,
    #[serde(rename = "queryCount")]
    pub query_count: u64,
}

/// POST /explore with one or more route plans. Fetches `id` internally.
#[cfg(feature = "reqwest")]
pub fn explore<I, S>(plans: I) -> Result<ExploreResponse>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let client = Client::new();
    let url = format!("{}/explore", AEDIFICIUM_BASE_URL);

    let id = get_id()?;
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

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/explore returned {}: {}", status, body);
    }

    let body: ExploreResponse = res.json().context("Failed to parse /explore response")?;
    Ok(body)
}

#[cfg(feature = "reqwest")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapConnectionEnd {
    pub room: u32,
    pub door: u32,
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
    pub rooms: Vec<u32>,
    #[serde(rename = "startingRoom")]
    pub starting_room: u32,
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
    let client = Client::new();
    let url = format!("{}/guess", AEDIFICIUM_BASE_URL);

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

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().unwrap_or_default();
        anyhow::bail!("/guess returned {}: {}", status, body);
    }

    let body: GuessResponse = res.json().context("Failed to parse /guess response")?;
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
