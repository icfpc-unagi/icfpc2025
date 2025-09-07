use itertools::Itertools;
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;

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

    // Solve using the shared solver and submit the guess
    let guess = icfpc2025::solve_no_marks::solve(n, &plans, &labels);
    judge.guess(&guess);
}
