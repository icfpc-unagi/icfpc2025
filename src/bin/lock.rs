use anyhow::Result;
use clap::{ArgAction, Parser};
use std::time::Duration;

use icfpc2025::lock as lockmod;

#[derive(Parser, Debug)]
#[command(name = "lock")]
#[command(about = "Acquire DB lock (lock_id=1)")]
struct Args {
    /// Forcefully take the lock by expiring any current holder
    #[arg(short = 'f', long = "force", action = ArgAction::SetTrue)]
    force: bool,

    /// Lock duration in seconds
    #[arg(short = 'd', long = "duration")]
    duration: Option<u64>,
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let ttl = Duration::from_secs(args.duration.unwrap_or(60));

    if args.force {
        // Forcefully expire any current lock before trying to acquire.
        lockmod::unlock("", true)?;
    }

    match lockmod::lock(ttl)? {
        Some(token) => {
            println!("{}", token);
            Ok(())
        }
        None => {
            // Acquisition failed because another active holder exists.
            std::process::exit(1);
        }
    }
}
