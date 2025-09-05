use chrono::Local;
use clap::{Parser, Subcommand};
use duct::cmd;
use fs2::FileExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

mod web;

#[derive(Serialize, Clone, Debug)]
pub struct InputData {
    pub file: String,
    pub url: String,
    pub seed: String,
    pub features: HashMap<String, String>,
}

pub fn get_inputs(set: &Path) -> (Vec<InputData>, Vec<String>) {
    let mut lines =
        std::io::BufReader::new(std::fs::File::open(set.join("in.csv")).unwrap()).lines();
    let mut list = vec![];
    let head = lines.next().unwrap().unwrap();
    let keys = head.split(',').collect_vec();
    assert_eq!(keys[0], "file");
    if keys.len() >= 2 {
        assert_eq!(keys[1], "seed");
    }
    for line in lines {
        let line = line.unwrap();
        let line = line.split(',').collect_vec();
        assert_eq!(line.len(), keys.len());
        list.push(InputData {
            file: line[0].to_owned(),
            seed: line.get(1).unwrap_or(&"0").to_string(),
            url: set
                .join("in")
                .join(format!("{}.txt", line[0]))
                .to_string_lossy()
                .to_string(),
            features: (2..line.len())
                .map(|i| (keys[i].to_owned(), line[i].to_owned()))
                .collect(),
        })
    }
    (list, keys.iter().map(|s| s.to_string()).collect())
}

#[derive(Serialize, Clone, Debug)]
pub struct ResultData {
    pub id: usize,
    pub date: String,
    pub msg: String,
    pub src: String,
    pub url: String,
    pub src_hash: String,
    pub src_id: usize,
    pub logs: Vec<BTreeMap<String, String>>,
}

pub fn get_result(set: &Path, inputs: &[InputData]) -> Vec<ResultData> {
    let mut result = vec![];
    let Ok(fs) = set.join("run").read_dir() else {
        return vec![];
    };
    for f in fs {
        let f = f.unwrap();
        if let Ok(id) = f.file_name().to_string_lossy().parse::<usize>() {
            let res: RunResult =
                toml::from_str(&std::fs::read_to_string(f.path().join("result.toml")).unwrap())
                    .unwrap();
            let url = f.path().join(&res.src).to_string_lossy().to_string();
            let logs = inputs
                .iter()
                .map(|i| res.logs.get(&i.file).cloned().unwrap_or_default())
                .collect();
            result.push(ResultData {
                id,
                date: res.date,
                msg: res.msg,
                src: res.src,
                url,
                src_hash: res.src_hash,
                src_id: 0,
                logs,
            })
        }
    }
    result.sort_by_key(|r| r.id);
    let mut src_ids = HashMap::new();
    for r in 0..result.len() {
        if result[r].src_hash.len() > 0 {
            if let Some(id) = src_ids.get(&result[r].src_hash) {
                result[r].src_id = *id;
            } else {
                result[r].src_id = result[r].id;
                src_ids.insert(result[r].src_hash.clone(), result[r].src_id);
            }
        } else {
            result[r].src_id = result[r].id;
        }
    }
    for i in 0..inputs.len() {
        let mut scores = vec![];
        for r in 0..result.len() {
            if let Some(v) = result[r].logs[i].get("score") {
                if let Ok(v) = v.parse::<f64>() {
                    scores.push((v, r));
                }
            }
        }
        scores.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut s = 0;
        while s < scores.len() {
            let mut t = s + 1;
            while t < scores.len() && scores[s].0 == scores[t].0 {
                t += 1;
            }
            for &(v, r) in &scores[s..t] {
                result[r].logs[i].insert(
                    "score_max".to_owned(),
                    ((1e9 * v / scores[scores.len() - 1].0).round() as i64).to_string(),
                );
                result[r].logs[i].insert(
                    "score_min".to_owned(),
                    ((1e9 * scores[0].0 / v).round() as i64).to_string(),
                );
                result[r].logs[i].insert(
                    "score_rank_max".to_owned(),
                    ((1e9
                        * (1.0
                            - ((scores.len() - t) as f64 + 0.5 * (t - s - 1) as f64)
                                / scores.len() as f64))
                        .round() as i64)
                        .to_string(),
                );
                result[r].logs[i].insert(
                    "score_rank_min".to_owned(),
                    ((1e9 * (1.0 - (s as f64 + 0.5 * (t - s - 1) as f64) / scores.len() as f64))
                        .round() as i64)
                        .to_string(),
                );
            }
            s = t;
        }
    }
    result
}

#[derive(Parser, Clone, Debug)]
struct Cli {
    #[command(subcommand)]
    cmd: SubCommand,
}

#[derive(Subcommand, Clone, Debug)]
enum SubCommand {
    Run(RunCommand),
    Start,
    Best(InputCommand),
    Clean(InputCommand),
}

#[derive(Parser, Clone, Debug)]
struct RunCommand {
    src: String,
    #[clap(short, long)]
    msg: Option<String>,
    #[clap(short, long)]
    input: Option<String>,
    #[clap(short, long)]
    parallel: Option<usize>,
}

#[derive(Parser, Clone, Debug)]
struct InputCommand {
    #[clap(short, long)]
    input: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    parallel: usize,
    default_set: String,
    listen: String,
    tester: String,
    default_key: String,
    vis: String,
    vis_ext: String,
    #[serde(default = "default_max_cases")]
    max_cases: usize,
}

fn default_max_cases() -> usize {
    100
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RunResult {
    date: String,
    msg: String,
    src: String,
    #[serde(default)]
    src_hash: String,
    #[serde(flatten)]
    logs: BTreeMap<String, BTreeMap<String, String>>,
}

fn compile(src: &Path, tmp: &TempDir) -> std::io::Result<String> {
    match src.extension().unwrap().to_str().unwrap() {
        "rs" => {
            cmd!(
                "cargo",
                "build",
                "--release",
                "--bin",
                src.file_stem().unwrap()
            )
            .run()?;
            cmd!(
                "cp",
                Path::new("target/release").join(src.file_stem().unwrap()),
                tmp.path().join("a.out")
            )
            .run()?;
            Ok(tmp.path().join("a.out").to_string_lossy().to_string())
        }
        "cpp" => {
            if cmd!(
                "g++-13",
                "-std=gnu++20",
                "-O2",
                "-DONLINE_JUDGE",
                "-DATCODER",
                "-mtune=native",
                "-march=native",
                "-fconstexpr-depth=2147483647",
                "-fconstexpr-loop-limit=2147483647",
                "-fconstexpr-ops-limit=2147483647",
                "-I/opt/boost/gcc/include",
                "-L/opt/boost/gcc/lib",
                format!("-I{}/ac-library", std::env::var("HOME").unwrap()),
                "-lgmpxx",
                "-lgmp",
                "-I/usr/include/eigen3",
                "-o",
                tmp.path().join("a.out"),
                src
            )
            // if cmd!(
            //     "g++",
            //     "-std=gnu++17",
            //     "-O2",
            //     "-DONLINE_JUDGE",
            //     "-I/opt/boost/gcc/include",
            //     "-L/opt/boost/gcc/lib",
            //     "-I$HOME/ac-library",
            //     "-o",
            //     tmp.path().join("a.out"),
            //     src
            // )
            .run()
            .is_ok()
            {
                Ok(tmp.path().join("a.out").to_string_lossy().to_string())
            } else {
                cmd!(
                    "clang++",
                    "-std=c++20",
                    "-Wall",
                    "-Wextra",
                    "-O2",
                    "-DONLINE_JUDGE",
                    "-DATCODER",
                    "-mtune=native",
                    "-march=native",
                    "-fconstexpr-depth=2147483647",
                    "-fconstexpr-steps=2147483647",
                    "-I/opt/boost/clang/include",
                    "-L/opt/boost/clang/lib",
                    format!("-I{}/ac-library", std::env::var("HOME").unwrap()),
                    "-I/usr/include/eigen3",
                    "-o",
                    tmp.path().join("a.out"),
                    src
                )
                .run()
                .expect("Compile Error");
                Ok(tmp.path().join("a.out").to_string_lossy().to_string())
            }
        }
        "py" => {
            cmd!("cp", src, tmp.path().join("Main.py")).run()?;
            cmd!("pypy3", "-m", "py_compile", "Main.py")
                .dir(tmp.path())
                .run()?;
            let _ = cmd!("pypy3", "Main.py", "ONLINE_JUDGE")
                .stdin_null()
                .stderr_null()
                .stdout_null()
                .run();
            std::fs::File::create(tmp.path().join("run.sh"))?.write_all(
                format!(
                    "#!/bin/sh
cd {}
pypy3 Main.py
",
                    tmp.path().display()
                )
                .as_bytes(),
            )?;
            Ok(format!("sh {}", tmp.path().join("run.sh").display()))
        }
        "go" => {
            cmd!("go", "build", "-o", tmp.path().join("a.out"), src).run()?;
            Ok(tmp.path().join("a.out").to_string_lossy().to_string())
        }
        ext => {
            panic!("{} is not supported", ext);
        }
    }
}

fn list_inputs(set: &Path) -> std::io::Result<Vec<PathBuf>> {
    Ok(get_inputs(set)
        .0
        .iter()
        .map(|i| i.url.clone().into())
        .collect())
}

fn next_run_id(set: &Path) -> std::io::Result<usize> {
    if !set.join("run").exists() {
        std::fs::create_dir(set.join("run"))?;
    }
    let mut id = 0;
    for f in set.join("run").read_dir()? {
        let f = f?;
        if let Ok(i) = f.file_name().to_string_lossy().parse::<usize>() {
            if id < i + 1 {
                id = i + 1;
            }
        }
    }
    Ok(id)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings: Settings = toml::from_str(&std::fs::read_to_string("ahc.toml")?)?;
    let cli = Cli::parse();
    match cli.cmd {
        SubCommand::Run(cmd) => {
            let parallel = cmd.parallel.unwrap_or(settings.parallel);
            let set_name = cmd.input.unwrap_or(settings.default_set.clone());
            let set = Path::new(&set_name);
            let src = Path::new(&cmd.src);
            let tmp = tempfile::tempdir()?;
            let solution = compile(src, &tmp).expect("Compile Error");
            if set.is_file() {
                std::fs::copy(set, "in.txt")?;
                cmd!(&settings.tester, &solution, set, "out.txt", "vis")
                    .run()
                    .expect(&format!(
                        "Failed to execute {} {} {} out.txt vis",
                        &settings.tester,
                        solution,
                        set.display()
                    ));
            } else {
                let mut inputs = list_inputs(set)?;
                inputs.reverse();
                let num_inputs = inputs.len();
                let inputs = Arc::new(Mutex::new(inputs));
                let msg = cmd
                    .msg
                    .unwrap_or_else(|| src.file_name().unwrap().to_string_lossy().to_string());
                let run_id = next_run_id(set)?;
                let run_dir = set.join("run").join(run_id.to_string());
                std::fs::create_dir(&run_dir)?;
                for dir in ["out", "err", "vis"] {
                    std::fs::create_dir(&run_dir.join(dir))?;
                }
                let copied_src = run_dir.join(src.file_name().unwrap());
                std::fs::copy(src, &copied_src)?;
                eprintln!("run_id = {}", run_id);
                let src_hash = {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(&std::fs::read(&copied_src)?);
                    hasher.finalize()
                };
                {
                    let file = std::fs::File::create(run_dir.join("result.toml"))?;
                    file.try_lock_exclusive()?;
                    let mut log = std::io::BufWriter::new(file);
                    writeln!(
                        log,
                        "date = \"{}\"",
                        Local::now().format("%m-%d %H:%M:%S").to_string()
                    )?;
                    writeln!(log, "msg = \"{}\"", msg)?;
                    writeln!(
                        log,
                        "src = \"{}\"",
                        copied_src.file_name().unwrap().to_string_lossy()
                    )?;
                    writeln!(log, "src_hash = \"{:x}\"", src_hash)?;
                    log.flush()?;
                    let log = Arc::new(Mutex::new(log));
                    let bar = indicatif::ProgressBar::new(inputs.lock().unwrap().len() as u64);
                    let threads = (0..parallel)
                        .map(|_| {
                            let inputs = inputs.clone();
                            let tester = settings.tester.clone();
                            let solution = solution.clone();
                            let run_dir = run_dir.clone();
                            let log = log.clone();
                            let bar = bar.clone();
                            std::thread::spawn(move || loop {
                                let input = if let Some(input) = inputs.lock().unwrap().pop() {
                                    input
                                } else {
                                    break;
                                };
                                let name = input.file_name().unwrap();
                                let _ = cmd!(
                                    &tester,
                                    &solution,
                                    &input,
                                    run_dir.join("out").join(name),
                                    run_dir.join("vis").join(input.file_stem().unwrap())
                                )
                                .stderr_path(run_dir.join("err").join(name))
                                .run();
                                bar.inc(1);
                                let mut err = BTreeMap::new();
                                for line in std::io::BufReader::new(
                                    std::fs::File::open(run_dir.join("err").join(name)).unwrap(),
                                )
                                .lines()
                                {
                                    let line = line.unwrap();
                                    if line.starts_with("!log ") {
                                        if let Some(p) = line[5..].find(' ') {
                                            let key = line[5..5 + p].to_owned();
                                            let val = line[5 + p + 1..].trim().to_owned();
                                            err.insert(key, val);
                                        }
                                    }
                                }
                                let mut log = log.lock().unwrap();
                                let _ = writeln!(
                                    log,
                                    "[{}]",
                                    input.file_stem().unwrap().to_string_lossy()
                                );
                                for (key, val) in err {
                                    let _ = writeln!(log, "{} = \"{}\"", key, val);
                                }
                                let _ = log.flush();
                            })
                        })
                        .collect::<Vec<_>>();
                    for t in threads {
                        t.join().unwrap();
                    }
                    bar.finish();
                }
                let result: RunResult =
                    toml::from_str(&std::fs::read_to_string(run_dir.join("result.toml"))?)?;
                let mut ac = 0;
                let mut total = 0.0;
                let mut min = (f64::INFINITY, String::new());
                let mut max = (f64::NEG_INFINITY, String::new());
                let mut max_time = (-1.0, String::new());
                let mut wa = String::new();
                for (name, log) in &result.logs {
                    if let Some(status) = log.get("status") {
                        if status == "AC" {
                            ac += 1;
                            if let Some(score) = log.get("score") {
                                if let Ok(score) = score.parse::<f64>() {
                                    total += score;
                                    if min.0 > score {
                                        min = (score, name.clone());
                                    }
                                    if max.0 < score {
                                        max = (score, name.clone());
                                    }
                                }
                            }
                        } else if wa.len() == 0 || &wa > name {
                            wa = name.clone();
                        }
                    }
                    if let Some(time) = log.get("time") {
                        if let Ok(time) = time.parse::<f64>() {
                            if max_time.0 < time {
                                max_time = (time, name.clone());
                            }
                        }
                    }
                }
                eprintln!("AC: {} / {}", ac, num_inputs);
                eprintln!("sum: {:.0}", total);
                eprintln!("avg: {:.0}", total / num_inputs as f64);
                eprintln!("min: {:.0} ({})", min.0, min.1);
                eprintln!("max: {:.0} ({})", max.0, max.1);
                eprintln!("max_time: {:.3} ({})", max_time.0, max_time.1);
                if wa.len() > 0 {
                    eprintln!("WA: {}", wa);
                }
                print!("{}", result.msg);
                for input in get_inputs(set).0 {
                    print!(
                        ",{}",
                        result.logs[&input.file]
                            .get("score")
                            .unwrap_or(&"-1".to_owned())
                    );
                }
                println!();
            }
        }
        SubCommand::Start => {
            eprintln!("Listening at http://{}", settings.listen);
            web::start(&settings)?;
        }
        SubCommand::Best(cmd) => {
            let set_name = cmd.input.unwrap_or(settings.default_set.clone());
            let set = Path::new(&set_name);
            let (inputs, _) = get_inputs(set);
            let result = get_result(set, &inputs);
            let best_dir = set.join("best");
            if !best_dir.exists() {
                std::fs::create_dir(&best_dir)?;
            }
            let minimize = settings.default_key.contains("min");
            let mut scores = vec![];
            for i in 0..inputs.len() {
                let mut best = !0;
                let mut best_score = -1.0;
                for r in 0..result.len() {
                    if result[r].logs[i].get("status") == Some(&"AC".to_owned()) {
                        if let Some(score) = result[r].logs[i].get("score") {
                            if let Ok(score) = score.parse::<f64>() {
                                if best == !0
                                    || minimize && best_score > score
                                    || !minimize && best_score < score
                                {
                                    best = r;
                                    best_score = score;
                                }
                            }
                        }
                    }
                }
                std::fs::copy(
                    set.join("run")
                        .join(result[best].id.to_string())
                        .join("out")
                        .join(format!("{}.txt", inputs[i].file)),
                    set.join("best").join(format!("{}.txt", inputs[i].file)),
                )?;
                scores.push(best_score);
            }
            eprintln!("sum: {:.0}", scores.iter().sum::<f64>());
            println!("best,{}", scores.iter().join(","));
        }
        SubCommand::Clean(cmd) => {
            let set_name = cmd.input.unwrap_or(settings.default_set.clone());
            let set = Path::new(&set_name);
            for run in set.join("run").read_dir()? {
                let run = run?;
                for dir in ["out", "err", "vis"] {
                    let path = run.path().join(dir);
                    if path.exists() {
                        eprintln!("rm -rf {}", path.display());
                        std::fs::remove_dir_all(path)?;
                    }
                }
            }
        }
    }
    Ok(())
}
