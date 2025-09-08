#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::solve_no_marks::{self, solve_cadical_multi};
use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
use solve_no_marks::SATSolver;
use std::path::Path;

fn balanced_plan_len(len: usize, rng: &mut ChaCha12Rng) -> Vec<usize> {
    let mut plan = Vec::with_capacity(len);
    for d in 0..6 {
        for _ in 0..(len / 6) {
            plan.push(d);
        }
    }
    plan.shuffle(rng);
    plan
}

fn main() {
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let n = judge.num_rooms();

    // Multiple plans setup
    let n_plans = 1;
    let len_plan = 18 * n;
    let mut rng = ChaCha12Rng::seed_from_u64(0xC0FF_EE42);

    let plans: Vec<Vec<usize>> = (0..n_plans)
        .map(|_| balanced_plan_len(len_plan, &mut rng))
        .collect();

    for plan in &plans {
        eprintln!("plan: {}", plan.iter().map(|d| d.to_string()).join(""));
    }

    let steps: Vec<Vec<(Option<usize>, usize)>> = plans
        .iter()
        .map(|p| p.iter().copied().map(|d| (None, d)).collect())
        .collect();
    let labels: Vec<Vec<usize>> = judge.explore(&steps);

    // ソルバ設定（環境変数で上書き可能）
    let solvers = [
        SATSolver {
            path: "/home/iwiwi/tmp/cryptominisat5".to_owned(),
            args: [
                "--threads=50",
                "-r",
                "1",
                "--presimp=1",
                "--occsimp=1",
                "--intree=1",
                "--transred=1",
                "--distill=1",
                "--distillbin=1",
                "--confbtwsimp=30000",
                "--confbtwsimpinc=1.3",
                "--sls=1",
                "--slstype=ccnr",
                "--slsgetphase=1",
                "--restart=auto",
                "--verb=2",
                "--breakid=1",
                "--breakideveryn=5",
                "--breakidmaxvars=300",
                "--breakidmaxcls=600",
                "--breakidmaxlits=3500",
                "--renumber=1",
                // "--polar=stable",
                "--autodisablegauss=true",
                "--bva=1",
            ]
            .map(|s| s.to_owned())
            .to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: ["--seed=0", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: ["--seed=1", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: ["--seed=2", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: ["--seed=3", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: ["--sat", "--walkinitially=true", "--walkeffort=100"]
                .map(|s| s.to_owned())
                .to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: ["--seed=0", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: ["--seed=1", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: ["--seed=2", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
        SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: ["--seed=3", "--sat"].map(|s| s.to_owned()).to_vec(),
        },
    ];

    let solvers = (0..50)
        .map(|seed| SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        })
        .collect_vec();

    let solvers = (0..25)
        .map(|seed| SATSolver {
            path: "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        })
        .chain((0..25).map(|seed| SATSolver {
            path: "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        }))
        .collect_vec();

    let dimacs_path = format!("sat_cnfs/tmp/{}.cnf", std::process::id());
    let dimacs_path = Path::new(&dimacs_path);
    if let Some(parent) = dimacs_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    // let guess = solve_cadical_multi(judge.num_rooms(), &plans, &labels, 100);
    // judge.guess(&guess);
    // return;

    let guess = icfpc2025::solve_no_marks::solve_portfolio(
        judge.num_rooms(),
        &plans,
        &labels,
        &solvers,
        dimacs_path,
    );
    judge.guess(&guess);
}

/*
=== metrics.json ===
{
    "combined_score": -26.86371646799671,
    "value_min_sec": 4.725222103996202,
    "value_p10_sec": 10.87974470500194,
    "value_p25_sec": 26.86371646799671,
    "value_median_sec": 300.0,
    "value_p50_sec": 300.0,
    "value_p75_sec": 300.0,
    "value_p90_sec": 300.0,
    "value_max_sec": 300.0,
    "timeout_sec": 300,
    "n_tests": 15,
    "n_workers": 1,
    "ac_count": 6,
    "score2_count": 6,
    "score2_rate": 0.4,
    "compile_time_sec": 2.4196691700053634,
    "binary_path": "/home/iwiwi/icfpc2025/target/release/eval_11bcc752_1534506",
    "score2_exec_median_sec": 21.20112343299843,
    "score2_exec_min_sec": 4.725222103996202
}
Saved correct.json and metri
 */
