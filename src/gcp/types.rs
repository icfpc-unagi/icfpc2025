use serde::{Deserialize, Serialize};

// Generic GCP types shared across services (service-agnostic).

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "type")]
    pub account_type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
