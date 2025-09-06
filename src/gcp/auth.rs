//! # GCP Authentication
//!
//! This module handles authentication with Google Cloud Platform using the
//! OAuth 2.0 flow for service accounts. It provides a function to obtain a
//! temporary access token that can be used to authorize API requests.

use anyhow::{Context, Result};
use cached::proc_macro::once;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

use crate::gcp::types::{AccessToken, ServiceAccount};

/// The Google OAuth2 token endpoint.
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Represents the claims in the JSON Web Token (JWT) used for authentication.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// The issuer of the token (the service account's email address).
    iss: String,
    /// The scope of the requested permissions.
    scope: String,
    /// The audience for the token (the token endpoint URL).
    aud: String,
    /// The expiration time of the token (Unix timestamp).
    exp: u64,
    /// The time the token was issued (Unix timestamp).
    iat: u64,
}

/// Fetches a GCP access token for the service account.
///
/// This function performs the server-to-server OAuth 2.0 flow:
/// 1. Downloads the service account key file from a predefined GCS path.
/// 2. Creates a JWT (JSON Web Token) with claims asserting the service account's identity
///    and the requested API scope.
/// 3. Signs the JWT using the service account's private key (RS256).
/// 4. Sends the signed JWT to the Google OAuth2 token endpoint.
/// 5. Receives an access token in exchange.
///
/// The `UNAGI_PASSWORD` environment variable must be set to locate the service account file.
///
/// # Returns
/// A `Result` containing the access token string if successful.
// #[cached(result = true)]
#[once(result = true)]
pub async fn get_access_token() -> Result<String> {
    // 1. Download the service account key file.
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

    // 2. Create the JWT claims.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let exp = now + 3600; // Token is valid for 1 hour.

    let claims = Claims {
        iss: service_account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: TOKEN_URL.to_string(),
        exp,
        iat: now,
    };

    // 3. Sign the JWT.
    let header = Header::new(Algorithm::RS256);
    let encoding_key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())?;
    let jwt = encode(&header, &claims, &encoding_key)?;

    // 4. Exchange the JWT for an access token.
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt),
    ];

    let response = client.post(TOKEN_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Failed to get access token: {}",
            error_text
        ));
    }

    // 5. Parse the response and return the token.
    let token_response: AccessToken = response.json().await?;
    Ok(token_response.access_token)
}
