//! # Common GCP Data Types
//!
//! This module defines common, service-agnostic data structures used for
//! interacting with Google Cloud Platform, particularly for authentication.

use serde::{Deserialize, Serialize};

/// Represents the structure of a GCP service account JSON key file.
///
/// This file contains the credentials needed for a service account to
/// authenticate with GCP APIs.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccount {
    /// The type of the key, typically "service_account".
    #[serde(rename = "type")]
    pub account_type: String,
    /// The GCP project ID associated with the service account.
    pub project_id: String,
    /// The unique ID of the private key.
    pub private_key_id: String,
    /// The PEM-encoded private key used to sign JWTs for authentication.
    pub private_key: String,
    /// The email address of the service account.
    pub client_email: String,
    /// The unique numeric ID for the client.
    pub client_id: String,
    /// The URI for the OAuth 2.0 authorization server.
    pub auth_uri: String,
    /// The URI for obtaining OAuth 2.0 access tokens.
    pub token_uri: String,
    /// The URL of the public x509 certificate for the auth provider.
    pub auth_provider_x509_cert_url: String,
    /// The URL of the public x509 certificate for the service account.
    pub client_x509_cert_url: String,
}

/// Represents an OAuth 2.0 access token returned by the GCP token URI.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    /// The access token string, used in the `Authorization` header of API requests.
    pub access_token: String,
    /// The type of the token, typically "Bearer".
    pub token_type: String,
    /// The number of seconds for which the token is valid.
    pub expires_in: u64,
}
