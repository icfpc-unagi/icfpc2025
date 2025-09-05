use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "gcp", about = "GCP utilities: instances/run/ls")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List GCE instances in a zone
    Instances {
        #[arg(long, default_value = "asia-northeast1-b")]
        zone: String,
        #[arg(long, default_value = "icfpc-primary")]
        project: String,
    },

    /// Create a GCE instance and optionally run a startup command
    Run {
        #[arg(long, default_value = "asia-northeast1-b")]
        zone: String,
        #[arg(long, default_value = "icfpc-primary")]
        project: String,
        #[arg(long, default_value = "c2d-standard-4")]
        machine_type: String,
        #[arg(name = "INSTANCE_NAME")]
        name: String,
        #[arg(name = "CMD", help = "Startup command to run (rest of args)")]
        cmd: Vec<String>,
    },

    /// List GCS objects like ls for a gs:// URL
    Ls {
        #[arg(short = 'l', long = "long")]
        long: bool,
        #[arg(short = 'R', long = "recursive")]
        recursive: bool,
        url: String,
    },

    /// Print a GCS object's content to stdout
    Cat { url: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Instances { zone, project } => commands::instances::run(&project, &zone).await,
        Commands::Run {
            zone,
            project,
            machine_type,
            name,
            cmd,
        } => commands::run::run(&project, &zone, &machine_type, &name, &cmd).await,
        Commands::Ls {
            long,
            recursive,
            url,
        } => commands::ls::run(long, recursive, &url).await,
        Commands::Cat { url } => commands::cat::run(&url).await,
    }
}

mod commands;
mod common;
