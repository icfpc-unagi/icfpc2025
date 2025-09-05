use anyhow::Result;

pub async fn run(
    project_id: &str,
    zone: &str,
    machine_type: &str,
    instance_name: &str,
    cmd: &[String],
) -> Result<()> {
    let startup_script = if cmd.is_empty() {
        None
    } else {
        Some(format!(
            "#!/bin/bash\nset -euxo pipefail\n{}\n",
            cmd.join(" ")
        ))
    };

    println!(
        "Creating GCE instance '{}' in zone '{}' (type: {})...",
        instance_name, zone, machine_type
    );

    let instance_request = icfpc2025::gcp::gce::create_instance_request(
        instance_name,
        project_id,
        zone,
        machine_type,
        startup_script.as_deref(),
    );

    let result = icfpc2025::gcp::gce::create_instance(project_id, zone, &instance_request).await?;
    println!(
        "Operation result: {}",
        serde_json::to_string_pretty(&result)?
    );
    Ok(())
}
