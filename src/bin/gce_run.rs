use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <instance-name> [zone]", args[0]);
        std::process::exit(1);
    }
    
    let instance_name = &args[1];
    let zone = args.get(2).map(|s| s.as_str()).unwrap_or("asia-northeast1-b");
    let project_id = "icfpc-primary";
    
    println!("Creating GCE instance '{}' in zone '{}'...", instance_name, zone);
    
    let instance_request = icfpc2025::gce::create_default_instance_request(instance_name);
    
    match icfpc2025::gce::create_instance(project_id, zone, &instance_request).await {
        Ok(result) => {
            println!("Instance creation initiated successfully!");
            println!("Operation result: {}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            eprintln!("Failed to create instance: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}