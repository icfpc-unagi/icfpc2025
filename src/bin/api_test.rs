use std::time::Instant;

use anyhow::Result;

fn main() -> Result<()> {
    // Run ping_root() 5 times sequentially and print elapsed ms
    for i in 0..5 {
        let start = Instant::now();
        icfpc2025::api::ping_root()?;
        let elapsed_ms = start.elapsed().as_millis();
        println!("#{i}: {elapsed_ms} ms");
    }
    Ok(())
}

