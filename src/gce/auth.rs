use anyhow::Result;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::gce::types::{AccessToken, ServiceAccount};

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
    let service_account_json = fs::read_to_string("secrets/service_account.json")?;
    let service_account: ServiceAccount = serde_json::from_str(&service_account_json)?;

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

    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", jwt.as_str()),
    ];

    let response = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Failed to get access token: {}", error_text));
    }

    let token_response: AccessToken = response.json().await?;
    Ok(token_response.access_token)
}

