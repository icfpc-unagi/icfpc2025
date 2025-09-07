#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case, unused_imports, dead_code)]

use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
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
    // マージ失敗により関数が消滅したので一旦コメントアウト

    /*
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
    let solver_bin = "/home/iwiwi/tmp/cryptominisat5";
    // let solver_args = ["--threads=63"];
    let solver_args = [
        "--threads=63",
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
    ];
    let solver_bin = "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64";
    let solver_args = [];

    let dimacs_path = format!("sat_cnfs/tmp/{}.cnf", std::process::id());
    let dimacs_path = Path::new(&dimacs_path);
    if let Some(parent) = dimacs_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    let solver_path = Path::new(&solver_bin);
    let guess = icfpc2025::solve_no_marks::solve_via_external_dimacs_streaming(
        n,
        &plans,
        &labels,
        solver_path,
        &solver_args,
        dimacs_path,
    );
    judge.guess(&guess);
    */
}
