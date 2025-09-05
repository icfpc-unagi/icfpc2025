use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::common::{last_segment, print_table};

pub async fn run(project_id: &str, zone: &str) -> Result<()> {
    let token = icfpc2025::gcp::get_access_token()
        .await
        .context("Failed to get access token")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://compute.googleapis.com/compute/v1/projects/{}/zones/{}/instances",
        project_id, zone
    );

    let res = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to call GCE list instances API")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        bail!("Failed to list instances (status {}): {}", status, body);
    }

    let json: Value = res.json().await.context("Failed to parse response JSON")?;

    let mut rows: Vec<[String; 5]> = Vec::new();
    if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
        for it in items {
            let status = it
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            let name = it
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let machine_type = it
                .get("machineType")
                .and_then(|v| v.as_str())
                .map(|s| last_segment(s).to_string())
                .unwrap_or_else(|| "".to_string());
            let zone_disp = it
                .get("zone")
                .and_then(|v| v.as_str())
                .map(|s| last_segment(s).to_string())
                .unwrap_or_else(|| zone.to_string());
            let external_ip = it
                .get("networkInterfaces")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|ni| ni.get("accessConfigs"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|ac| ac.get("natIP"))
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string();

            rows.push([status, name, machine_type, zone_disp, external_ip]);
        }
    }

    print_table(
        &["Status", "Name", "Machine Type", "Zone", "External IP"],
        &rows,
    );
    Ok(())
}
