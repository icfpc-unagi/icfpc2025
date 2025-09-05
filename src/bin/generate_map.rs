use anyhow::bail;
use clap::Parser;
use clap::ValueEnum;
use icfpc2025::{mapgen, svg};
use std::fs;

#[derive(Parser)]
struct Cli {
    /// Number of rooms.
    #[clap(long, short = 'n', default_value_t = 10)]
    n_rooms: usize,
    /// Path to output file. If not provided, outputs to stdout.
    #[clap(long, short = 'o', default_value = "")]
    output: String,
    /// File format: json or svg. If not provided, infers from output file extension.
    #[clap(long, short = 'f', default_value = "unspecified")]
    format: Format,
    #[clap(long, short = 'c', default_value_t = false)]
    compact: bool,
    #[clap(long, short = 't', default_value = "random")]
    r#type: String,
    #[clap(long, short = 's')]
    seed: Option<u64>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Format {
    #[default]
    Unspecified,
    Json,
    Svg,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let map = match args.r#type.as_str() {
        "random" => mapgen::random::generate_as_api_map(args.n_rooms, args.seed),
        other => bail!("Unknown type: {}", other),
    };
    // Infer format from output file extension if not specified.
    let format = if args.format == Format::Unspecified {
        if args.output.ends_with(".json") {
            Format::Json
        } else if args.output.ends_with(".svg") {
            Format::Svg
        } else if args.output.is_empty() {
            Format::Json
        } else {
            bail!("Cannot infer format from output file extension. Specify format with -f option.")
        }
    } else {
        args.format.clone()
    };

    use std::io::Write;

    let mut w: Box<dyn Write> = if args.output.is_empty() {
        Box::new(std::io::stdout())
    } else {
        Box::new(fs::File::create(&args.output)?)
    };

    match format {
        Format::Json => {
            if args.compact {
                serde_json::to_writer(&mut w, &map)?;
            } else {
                serde_json::to_writer_pretty(&mut w, &map)?;
            }
        }
        Format::Svg => {
            let svg_content = svg::render(&map);
            w.write_all(svg_content.as_bytes())?;
        }
        Format::Unspecified => {
            unreachable!()
        }
    }
    Ok(())
}
