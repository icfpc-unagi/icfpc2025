use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::gcp::types::{AccessToken, ServiceAccount};

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

pub async fn get_access_token() -> Result<String> {
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let sa_url = format!(
        "https://storage.googleapis.com/icfpc2025-data/{}/service_account.json",
        unagi_password
    );

    let client = reqwest::Client::new();
    let service_account_json = client
        .get(sa_url)
        .send()
        .await
        .context("Failed to download service_account.json")?
        .error_for_status()
        .context("Failed to download service_account.json: HTTP error")?
        .text()
        .await
        .context("Failed to read service_account.json body")?;

    let service_account: ServiceAccount =
        serde_json::from_str(&service_account_json).context("Invalid service_account.json")?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let exp = now + 3600;

    let claims = Claims {
        iss: service_account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: TOKEN_URL.to_string(),
        exp,
        iat: now,
    };

    let header = Header::new(Algorithm::RS256);
    let encoding_key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())?;

    let jwt = encode(&header, &claims, &encoding_key)?;

    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", jwt.as_str()),
    ];

    let response = client.post(TOKEN_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Failed to get access token: {}",
            error_text
        ));
    }

    let token_response: AccessToken = response.json().await?;
    Ok(token_response.access_token)
}
