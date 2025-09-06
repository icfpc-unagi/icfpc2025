use clap::Parser;

/// Tester tool for running a command with input/output redirection.
#[derive(Parser, Debug)]
struct Cli {
    /// The shell command to run (e.g., the solution binary)
    cmd: String,
    /// Path to the input file
    input: String,
    /// Path to the output file (will be created)
    output: String,
    /// Path to the visualization file (unused)
    vis: String,
}

fn main() {
    let cli = Cli::parse();
    let input_file =
        std::fs::File::open(&cli.input).unwrap_or_else(|_| panic!("No such input: {}", cli.input));
    let output_file = std::fs::File::create(&cli.output)
        .unwrap_or_else(|_| panic!("Cannot create {}", cli.output));
    let stime = std::time::SystemTime::now();
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cli.cmd)
        .stdin(std::process::Stdio::from(input_file))
        .stdout(std::process::Stdio::from(output_file))
        .stderr(std::process::Stdio::inherit())
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute command: {}", cli.cmd));
    let t = std::time::SystemTime::now().duration_since(stime).unwrap();
    let ms = t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9;
    eprintln!("!log time {:.3}", ms);
    if !status.success() {
        if status.code() == Some(124) {
            eprintln!("!log status TLE");
        } else {
            eprintln!("!log status RE");
        }
    }
}
