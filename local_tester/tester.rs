use tools::*;

use clap::Parser;
use std::io::Write;

#[derive(Parser, Debug)]
struct Cli {
    cmd: String,
    input: String,
    output: String,
    vis: String,
}

fn main() {
    let cli = Cli::parse();
    {
        let input_file = std::fs::File::open(&cli.input).expect(&format!("No such input: {}", cli.input));
        let output_file = std::fs::File::create(&cli.output).expect(&format!("Cannot create {}", cli.output));
        let stime = std::time::SystemTime::now();
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("ulimit -Sv 4000000; timeout --foreground 60s {}", cli.cmd)) // for standard problem
            // .arg(format!("ulimit -Sv 4000000; timeout --foreground 60s ../tools/target/release/tester {}", cli.cmd)) // for interactive problem
            .stdin(std::process::Stdio::from(input_file))
            .stdout(std::process::Stdio::from(output_file))
            .stderr(std::process::Stdio::inherit())
            .status()
            .expect(&format!("Failed to execute command: {}", cli.cmd));
        let t = std::time::SystemTime::now().duration_since(stime).unwrap();
        let ms = t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9;
        eprintln!("!log time {:.3}", ms);
        if !status.success() {
            if status.code() == Some(124) {
                eprintln!("!log status TLE");
            } else {
                eprintln!("!log status RE");
            }
            return;
        }
    };
    let input = std::fs::read_to_string(&cli.input).unwrap();
    let output = std::fs::read_to_string(&cli.output).unwrap();
    let input = parse_input(&input);
    match parse_output(&input, &output) {
        Ok(out) => {
            let (score, err, svg) = vis_default(&input, &out);
            if err.len() > 0 {
                eprintln!("{}", err);
                eprintln!("!log status WA");
                return;
            }
            eprintln!("!log status AC");
            eprintln!("!log score {}", score);
            std::fs::File::create(&format!("{}.svg", cli.vis))
                .expect(&format!("Cannot create {}", cli.vis))
                .write_all(svg.as_bytes())
                .unwrap();
        }
        Err(err) => {
            eprintln!("{}", err);
            eprintln!("!log status WA");
        }
    }
}
