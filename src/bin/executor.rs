use anyhow::Result;
use clap::Parser;
use std::thread;
use std::time::Duration;

use icfpc2025::executor as exec;

#[derive(Parser, Debug)]
#[command(name = "executor", about = "Task executor loop")]
struct Args {
    /// Sleep milliseconds when no task is available
    #[arg(long = "sleep-ms", default_value_t = 1000)]
    sleep_ms: u64,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    loop {
        match exec::acquire_task()? {
            Some(task) => {
                // Optionally heartbeat could be added with a separate thread calling extend_lock.
                let (score, duration_ms) = exec::run_task(&task)?;
                exec::update_task(&task, score, duration_ms)?;
            }
            None => {
                thread::sleep(Duration::from_millis(args.sleep_ms));
            }
        }
    }
}
