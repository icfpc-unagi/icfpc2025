use anyhow::bail;
use clap::Parser;
use icfpc2025::mapgen;

#[derive(Parser)]
struct Cli {
    #[clap(long, short = 'n', default_value = "10")]
    n_rooms: usize,
    #[clap(long, short = 's')]
    seed: Option<u64>,
    #[clap(long, short = 'c', default_value_t = false)]
    compact: bool,
    #[clap(long, short = 't', default_value = "random")]
    r#type: String,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let map = match args.r#type.as_str() {
        "random" => mapgen::random::generate_as_api_map(args.n_rooms, args.seed),
        other => bail!("Unknown type: {}", other),
    };
    if args.compact {
        println!("{}", serde_json::to_string(&map).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
    }
    Ok(())
}
