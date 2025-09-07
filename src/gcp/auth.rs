//! # GCP Authentication
//!
//! This module handles authentication with Google Cloud Platform using the
//! OAuth 2.0 flow for service accounts. It provides a function to obtain a
//! temporary access token that can be used to authorize API requests.

use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::client::CLIENT;
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
/// Cache for the downloaded and parsed Service Account JSON.
/// This avoids refetching the key material on every token request.
static SA_CACHE: Lazy<Mutex<Option<ServiceAccount>>> = Lazy::new(|| Mutex::new(None));

/// Cache for access tokens with a short lifetime to avoid frequent token endpoint calls.
/// We cache for at most 5 minutes and never beyond the token's actual expiry (minus a safety margin).
struct TokenCache {
    token: String,
    fetched_at: Instant,
    expires_at: Instant,
}

static TOKEN_CACHE: Lazy<Mutex<Option<TokenCache>>> = Lazy::new(|| Mutex::new(None));

pub async fn get_access_token() -> Result<String> {
    // 0. Check token cache: valid if fetched within 5 minutes AND not near expiry (60s margin)
    if let Some(c) = TOKEN_CACHE.lock().unwrap().as_ref() {
        let now = Instant::now();
        let within_5m = now.duration_since(c.fetched_at) < Duration::from_secs(5 * 60);
        let not_near_expiry = now + Duration::from_secs(60) < c.expires_at;
        if within_5m && not_near_expiry {
            return Ok(c.token.clone());
        }
    }

    // 1. Get or download the service account key file (cacheable)
    let service_account = if let Some(sa) = SA_CACHE.lock().unwrap().clone() {
        sa
    } else {
        let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
        let sa_url = format!(
            "https://storage.googleapis.com/icfpc2025-data/{}/service_account.json",
            unagi_password
        );

        let client = &*CLIENT;
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

        let sa: ServiceAccount =
            serde_json::from_str(&service_account_json).context("Invalid service_account.json")?;
        *SA_CACHE.lock().unwrap() = Some(sa.clone());
        sa
    };

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

    let client = &*CLIENT;
    let response = client.post(TOKEN_URL).form(&params).send().await?;
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "<no body>".into());
        anyhow::bail!("Failed to get access token: {}", error_text);
    }
    let token_response: AccessToken = response.json().await?;
    let token = token_response.access_token;
    // Cache with expiry and fetched_at timestamps
    let fetched_at = Instant::now();
    let expires_at = fetched_at + Duration::from_secs(token_response.expires_in.saturating_sub(0));
    *TOKEN_CACHE.lock().unwrap() = Some(TokenCache {
        token: token.clone(),
        fetched_at,
        expires_at,
    });
    Ok(token)
}
