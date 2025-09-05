use anyhow::{Context, Result, bail};
use icfpc2025::api;
use icfpc2025::problems;
use serde_json::Value;
use std::env;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let sub = args.next().unwrap_or_default();
    let json_arg = args.next().unwrap_or_default();

    if sub.is_empty() || json_arg.is_empty() {
        eprintln!("Usage: post <select|explore|guess> '<json>'");
        bail!("missing arguments");
    }

    match sub.as_str() {
        "select" => handle_select(&json_arg),
        "explore" => handle_explore(&json_arg),
        "guess" => handle_guess(&json_arg),
        _ => {
            eprintln!("Unknown subcommand: {}", sub);
            bail!("unknown subcommand")
        }
    }
}

fn handle_select(json_arg: &str) -> Result<()> {
    let v: Value = serde_json::from_str(json_arg).context("invalid JSON for select")?;
    let obj = v.as_object().context("select expects a JSON object")?;

    let problem_name = obj
        .get("problemName")
        .and_then(|v| v.as_str())
        .context("select requires field 'problemName': string")?;

    // Validate problem name using local list.
    if problems::get_problem(problem_name).is_none() {
        bail!("unknown problemName: {}", problem_name);
    }

    // Validate optional id if provided matches fetched id.
    let input_id = obj.get("id").and_then(|v| v.as_str());
    let id = api::get_id()?;
    if let Some(given) = input_id
        && given != id
    {
        bail!("provided id does not match local id.json");
    }

    let selected = api::select(problem_name)?;
    let out = serde_json::json!({ "problemName": selected });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}

fn validate_plan(s: &str) -> bool {
    s.bytes()
        .all(|b| matches!(b, b'0' | b'1' | b'2' | b'3' | b'4' | b'5'))
}

fn handle_explore(json_arg: &str) -> Result<()> {
    let v: Value = serde_json::from_str(json_arg).context("invalid JSON for explore")?;
    let obj = v.as_object().context("explore expects a JSON object")?;

    // Validate optional id if provided.
    let input_id = obj.get("id").and_then(|v| v.as_str());
    let id = api::get_id()?;
    if let Some(given) = input_id
        && given != id
    {
        bail!("provided id does not match local id.json");
    }

    let plans_v = obj
        .get("plans")
        .context("explore requires field 'plans': [string]")?;
    let plans_arr = plans_v.as_array().context("'plans' must be an array")?;
    let mut plans: Vec<Vec<usize>> = Vec::with_capacity(plans_arr.len());
    for (i, p) in plans_arr.iter().enumerate() {
        let s = p
            .as_str()
            .with_context(|| format!("plans[{}] must be a string", i))?;
        if !validate_plan(s) {
            bail!(
                "plans[{}] contains non-digit or out-of-range digit (allowed: 0-5)",
                i
            );
        }
        plans.push(
            s.chars()
                .map(|c| c.to_digit(10).unwrap() as usize)
                .collect(),
        );
    }

    let resp = api::explore(plans)?;
    let out = serde_json::json!({
        "results": resp.results,
        "queryCount": resp.query_count,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}

fn handle_guess(json_arg: &str) -> Result<()> {
    let v: Value = serde_json::from_str(json_arg).context("invalid JSON for guess")?;
    let obj = v.as_object().context("guess expects a JSON object")?;

    // Validate optional id if provided.
    let input_id = obj.get("id").and_then(|v| v.as_str());
    let id = api::get_id()?;
    if let Some(given) = input_id
        && given != id
    {
        bail!("provided id does not match local id.json");
    }

    // Deserialize map using the API types.
    #[derive(serde::Deserialize)]
    struct GuessIn {
        map: api::Map,
    }
    let guess: GuessIn = serde_json::from_value(v).context("'map' is required for guess")?;
    validate_map(&guess.map)?;

    let correct = api::guess(&guess.map)?;
    let out = serde_json::json!({ "correct": correct });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}

fn validate_map(map: &api::Map) -> Result<()> {
    // rooms must be 2-bit integers 0..=3
    for (i, &v) in map.rooms.iter().enumerate() {
        if v > 3 {
            bail!("rooms[{}] must be in 0..=3 (2-bit)", i);
        }
    }
    // starting_room must be valid index
    if (map.starting_room as usize) >= map.rooms.len() {
        bail!("startingRoom is out of range");
    }
    let n = map.rooms.len();
    // connections: room indices valid and door numbers 0..=5
    for (i, c) in map.connections.iter().enumerate() {
        for (side, end) in [("from", &c.from), ("to", &c.to)] {
            if end.room >= n {
                bail!("connections[{}].{}.room out of range", i, side);
            }
            if end.door > 5 {
                bail!("connections[{}].{}.door must be in 0..=5", i, side);
            }
        }
    }
    Ok(())
}
