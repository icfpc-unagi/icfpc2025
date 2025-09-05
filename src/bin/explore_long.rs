use anyhow::{bail, Context, Result};
use icfpc2025::api;
use rand::Rng;
use std::env;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let n_str = args.next().unwrap_or_default();

    if n_str.is_empty() {
        eprintln!("Usage: explore_long <n>");
        bail!("missing argument: n");
    }

    let n: usize = n_str
        .parse()
        .with_context(|| format!("invalid n: {} (expected integer)", n_str))?;

    // Generate a random plan string of length n with digits 0..=5.
    let mut rng = rand::rng();
    let mut plan = String::with_capacity(n);
    for _ in 0..n {
        let d: u8 = rng.random_range(0..=5);
        plan.push((b'0' + d) as char);
    }

    // Call explore with a single plan and print the response similar to post.rs.
    let resp = api::explore(std::iter::once(plan.as_str()))?;
    let results_length: Vec<usize> = resp.results.iter().map(|v| v.len()).collect();
    let out = serde_json::json!({
        "resultsLength": results_length,
        "queryCount": resp.query_count,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
