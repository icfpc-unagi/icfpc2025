//! # GCE API Client Logic
//!
//! This module contains the core client function for making API requests to
//! Google Compute Engine.

use anyhow::Result;
use serde_json::Value;

use crate::client::CLIENT;
use crate::gcp::gce::types::InstanceRequest;
use crate::gcp::get_access_token;

/// The base URL for the Google Compute Engine v1 API.
const GCE_API_BASE: &str = "https://compute.googleapis.com/compute/v1";

/// Creates a new GCE virtual machine instance.
///
/// This function constructs and sends a POST request to the GCE `instances.insert`
/// API endpoint. It handles authentication by acquiring an access token.
///
/// # Arguments
/// * `project_id` - The GCP project ID in which to create the instance.
/// * `zone` - The GCP zone in which to create the instance.
/// * `instance_request` - A struct containing the detailed configuration for the new instance.
///
/// # Returns
/// A `Result` containing the JSON response from the GCE API as a `serde_json::Value`.
/// The response typically represents a long-running Operation resource.
pub async fn create_instance(
    project_id: &str,
    zone: &str,
    instance_request: &InstanceRequest,
) -> Result<Value> {
    // Authenticate to get a bearer token.
    let token = get_access_token().await?;

    let client = &*CLIENT;
    let url = format!(
        "{}/projects/{}/zones/{}/instances",
        GCE_API_BASE, project_id, zone
    );

    // Send the authorized POST request with the instance configuration as JSON.
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(instance_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Failed to create instance: {}", error_text));
    }

    // Return the raw JSON response from the API.
    let result: Value = response.json().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    // ネットワークアクセス前提。UNAGI_PASSWORD が設定されている必要があります。
    #[tokio::test]
    #[ignore]
    async fn can_get_access_token() -> Result<()> {
        let token = crate::gcp::get_access_token().await?;
        assert!(!token.is_empty());
        Ok(())
    }
}
