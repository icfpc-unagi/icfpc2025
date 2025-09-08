use std::io::Read as _;

use anyhow::{Context as _, Result};

fn main() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let map = serde_json::from_str::<icfpc2025::judge::JsonIn>(input.trim())
        .context("invalid JSON")?
        .map
        .context("missing map")?;
    let output = icfpc2025::layered::reduce_graph(&map)?;
    let json_out = serde_json::to_string(&output).unwrap();
    println!("{}", json_out);
    Ok(())
}
