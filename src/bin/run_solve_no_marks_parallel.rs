use clap::Parser;
use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;

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

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, short = 'j', default_value_t = 5)]
    threads: usize,
    #[clap(long, default_value_t = 8192)]
    min_tasks: usize,
    #[clap(long, default_value_t = 4)]
    max_depth: usize,
}

fn main() {
    let Args {
        mut threads,
        min_tasks,
        max_depth,
    } = Args::parse();
    if threads == 0 {
        threads = 1;
    }
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let n = judge.num_rooms();

    // Multiple plans setup
    let n_plans = 3;
    let len_plan = 6 * n;
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

    // Build a task list by fixing a prefix of edges starting from time 0 of plan 0.
    // Prefix depth escalates (1->2->3) until we have enough tasks to saturate threads.
    type TaskPrefix = Vec<(usize, usize, usize, Option<usize>)>;
    type Tasks = Vec<TaskPrefix>;
    let u0 = labels[0][0];
    // Helper to push all (v,f) pairs for a given (u,e,h) into base prefixes.
    let expand_with = |bases: Tasks, u: usize, e: usize, h: usize| {
        let vcands: Vec<usize> = (0..n).filter(|&v| v % 4 == h).collect();
        let mut out: Tasks = Vec::new();
        for base in bases {
            for &v in &vcands {
                for f in 0..6 {
                    let mut p = base.clone();
                    p.push((u, e, v, Some(f)));
                    out.push(p);
                }
            }
        }
        out
    };

    // Start with depth=1
    let e0 = plans[0][0];
    let h0 = labels[0][1];
    let mut tasks: Tasks = expand_with(vec![Vec::new()], u0, e0, h0);
    // Increase depth until we have enough tasks or hit limits
    let want = min_tasks.max(threads.saturating_mul(64));
    let max_k = plans[0].len().min(max_depth);
    let mut k = 1usize;
    while tasks.len() < want && k < max_k {
        let e_k = plans[0][k];
        let h_k = labels[0][k + 1];
        let mut bases = Vec::new();
        std::mem::swap(&mut bases, &mut tasks);
        let mut next = Vec::new();
        for base in bases {
            let u_cur = base.last().unwrap().2;
            let mut expanded = expand_with(vec![base], u_cur, e_k, h_k);
            next.append(&mut expanded);
        }
        tasks = next;
        k += 1;
    }
    eprintln!(
        "prepared {} parallel tasks (prefix depth {})",
        tasks.len(),
        tasks.first().map(|t| t.len()).unwrap_or(0)
    );

    // Use a worker pool limited by --threads.
    let (tx, rx) = mpsc::channel();
    let next = Arc::new(AtomicUsize::new(0));
    let tasks = Arc::new(tasks);
    let plans_arc = Arc::new(plans);
    let labels_arc: Arc<Vec<Vec<usize>>> = Arc::new(labels);
    for _ in 0..threads {
        let tx = tx.clone();
        let next = Arc::clone(&next);
        let tasks = Arc::clone(&tasks);
        let plans = Arc::clone(&plans_arc);
        let labels = Arc::clone(&labels_arc);
        thread::spawn(move || {
            loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= tasks.len() {
                    break;
                }
                let prefix = &tasks[i];
                if let Some(guess) = icfpc2025::solve_no_marks::solve_with_edge_prefix_fixed(
                    n, &plans, &labels, prefix,
                ) {
                    let _ = tx.send(guess);
                    break; // stop after first success in this worker
                }
            }
        });
    }
    drop(tx); // ensure recv unblocks if all branches are UNSAT

    let guess = rx
        .recv()
        .expect("no parallel branch produced a valid guess");
    judge.guess(&guess);
    let _ = io::stdout().flush();

    std::process::exit(0);
}
