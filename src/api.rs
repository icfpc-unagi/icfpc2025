use anyhow::Context;

#[cfg(feature = "reqwest")]
use reqwest::Client;

/// Fetches `id.json` from the same directory as `bearer.txt`.
///
/// The path is: `https://storage.googleapis.com/icfpc2025-data/{UNAGI_PASSWORD}/id.json`.
#[cfg(feature = "reqwest")]
pub async fn get_id_async() -> anyhow::Result<Vec<u8>> {
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = Client::new();
    let res = client
        .get(format!(
            "https://storage.googleapis.com/icfpc2025-data/{}/id.json",
            unagi_password
        ))
        .send()
        .await
        .context("Failed to get id.json")?;
    res.bytes()
        .await
        .map(|b| b.to_vec())
        .context("Failed to read id.json body")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    // Runs only when explicitly enabled (e.g., `make test/unagi`).
    // Requires `UNAGI_PASSWORD` to be set to access the remote object.
    #[ignore]
    #[tokio::test]
    async fn sha1_of_id_json_matches_expected() -> Result<()> {
        // If UNAGI_PASSWORD isn't set, skip gracefully.
        if std::env::var("UNAGI_PASSWORD").is_err() {
            eprintln!("UNAGI_PASSWORD not set; skipping sha1 check for id.json");
            return Ok(());
        }

        let bytes = get_id_async().await?;

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
