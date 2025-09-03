use anyhow::{Context, Result};
use serde_json::Value;
use std::env;

const DEFAULT_PROJECT: &str = "icfpc-primary";
const DEFAULT_ZONE: &str = "asia-northeast1-b";

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Usage: gce_list_instances [zone]
    if args.len() > 2 {
        eprintln!("Usage: {} [zone]", args[0]);
        std::process::exit(1);
    }

    let zone = args.get(1).map(|s| s.as_str()).unwrap_or(DEFAULT_ZONE);
    let project_id = DEFAULT_PROJECT;

    let token = icfpc2025::gce::get_access_token()
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
        eprintln!("Failed to list instances (status {}): {}", status, body);
        std::process::exit(1);
    }

    let json: Value = res.json().await.context("Failed to parse response JSON")?;

    // Build rows: Status, Name, MachineType, Zone, ExternalIP
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

    // Render table
    print_table(
        &["Status", "Name", "Machine Type", "Zone", "External IP"],
        &rows,
    );

    Ok(())
}

fn last_segment(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

fn print_table(headers: &[&str; 5], rows: &[[String; 5]]) {
    let mut widths = [0usize; 5];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = widths[i].max(display_width(h));
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }

    // Header
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{:width$}", h, width = widths[i]);
    }
    println!();

    // Separator
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{}", "-".repeat(*w));
    }
    println!();

    // Rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                print!("  ");
            }
            print!("{:width$}", cell, width = widths[i]);
        }
        println!();
    }
}

fn display_width(s: &str) -> usize {
    // Rough width calculation; monospace-friendly
    s.chars().count()
}
