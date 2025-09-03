use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "gce_run",
    about = "Create a GCE instance and optionally run a startup command"
)]
struct Args {
    #[arg(long, default_value = "asia-northeast1-b")]
    zone: String,

    #[arg(long, default_value = "c2d-standard-4")]
    machine_type: String,

    #[arg(name = "INSTANCE_NAME")]
    name: String,

    #[arg(name = "CMD", help = "Startup command to run (rest of args)")]
    cmd: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let project_id = "icfpc-primary";
    let instance_name = &args.name;
    let zone = &args.zone;

    // Build startup script if commands provided
    let startup_script = if args.cmd.is_empty() {
        None
    } else {
        Some(format!(
            "#!/bin/bash\nset -euxo pipefail\n{}\n",
            args.cmd.join(" ")
        ))
    };

    println!(
        "Creating GCE instance '{}' in zone '{}' (type: {})...",
        instance_name, zone, args.machine_type
    );

    let instance_request = icfpc2025::gce::create_instance_request(
        instance_name,
        project_id,
        zone,
        &args.machine_type,
        startup_script.as_deref(),
    );

    match icfpc2025::gce::create_instance(project_id, zone, &instance_request).await {
        Ok(result) => {
            println!("Instance creation initiated successfully!");
            println!(
                "Operation result: {}",
                serde_json::to_string_pretty(&result)?
            );
        }
        Err(e) => {
            eprintln!("Failed to create instance: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
