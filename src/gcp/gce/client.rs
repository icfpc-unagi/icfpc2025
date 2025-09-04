use anyhow::Result;
use serde_json::Value;

use crate::gcp::gce::types::InstanceRequest;
use crate::gcp::get_access_token;

const GCE_API_BASE: &str = "https://compute.googleapis.com/compute/v1";

pub async fn create_instance(
    project_id: &str,
    zone: &str,
    instance_request: &InstanceRequest,
) -> Result<Value> {
    let token = get_access_token().await?;

    let client = reqwest::Client::new();
    let url = format!(
        "{}/projects/{}/zones/{}/instances",
        GCE_API_BASE, project_id, zone
    );

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

    let result: Value = response.json().await?;
    Ok(result)
}
