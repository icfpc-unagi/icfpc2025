#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]
use icfpc2025::{judge::*, *};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::prelude::*;

fn coverage(local_judge: &LocalJudge, plan: &Vec<usize>) -> (f32, f32, f32) {
    let mut cnt = vec![[0; 6]; local_judge.num_rooms()];
    let mut u = 0;

    for &d in plan {
        cnt[u][d] += 1;
        u = local_judge.graph[u][d];
    }

    let total_covered = cnt
        .iter()
        .map(|x| x.iter().filter(|&&b| b >= 1).count())
        .sum::<usize>();
    let ratio_covered = total_covered as f32 / (local_judge.num_rooms() * 6) as f32;

    let normalized_entropy = cnt
        .iter()
        .map(|x| {
            let s = x.iter().sum::<usize>() as f32;
            if s == 0.0 {
                0.0
            } else {
                -x.iter()
                    .filter(|&&c| c >= 1)
                    .map(|&c| {
                        let p = c as f32 / s;
                        p * p.log2()
                    })
                    .sum::<f32>()
            }
        })
        .sum::<f32>()
        / (local_judge.num_rooms() as f32 * 6.0f32.log2());

    let perfect_covered = if ratio_covered == 1.0 { 1.0 } else { 0.0 };

    (ratio_covered, normalized_entropy, perfect_covered)
}

fn generate_plan(num_rooms: usize, n_seeds: usize) -> Vec<usize> {
    let mut rng = rand::rng();

    let mut local_judges = (0..n_seeds)
        .map(|i| LocalJudge::new("random", num_rooms, i as u64))
        .collect_vec();

    /*
        let mut vis = vec![vec![[0; 6]; num_rooms]; n_seeds];
        let mut pos = vec![0; n_seeds];
    */

    let mut plans = vec![];
    let plan_len = 18 * num_rooms;
    for i in 0..plan_len {
        if false && rng.random_range(0..20) == 0 {
            plans.push(rng.random_range(0..6));
            continue;
        }

        let mut best = (OrderedFloat(0.0), OrderedFloat(0.0), !0);
        let mut order = (0..6).collect_vec();
        order.shuffle(&mut rng);
        for &d in &order {
            if (i >= 1 && d == plans[i - 1]) || (i >= 2 && d == plans[i - 2]) {
                // tos sensei heuristic
                // continue;
            }

            let mut tmp_plans = plans.clone();
            tmp_plans.push(d);
            let evals = local_judges
                .iter()
                .map(|lj| coverage(lj, &tmp_plans))
                .collect_vec();
            let tmp_coverage = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds as f32;
            let tmp_entropy = evals.iter().map(|(_, a, _)| a).sum::<f32>() / n_seeds as f32;

            best = best.max((OrderedFloat(tmp_coverage), OrderedFloat(tmp_entropy), d));
        }
        eprintln!("Coverage {} {} {}", i, best.0, best.1);
        plans.push(best.2);
    }

    // 各数字の出てくる回数を表示
    let mut cnt = [0; 6];
    for &d in &plans {
        cnt[d] += 1;
    }
    eprintln!("Count: {}", cnt.iter().map(|&c| c.to_string()).join(" "));

    eprintln!("{}", plans.iter().map(|d| d.to_string()).join(""));

    plans
}

fn generate_plan_v2(num_rooms: usize, n_seeds: usize) -> Vec<usize> {
    let mut rng = rand::rng();

    let mut local_judges = (0..n_seeds)
        .map(|i| LocalJudge::new("random", num_rooms, i as u64))
        .collect_vec();

    /*
        let mut vis = vec![vec![[0; 6]; num_rooms]; n_seeds];
        let mut pos = vec![0; n_seeds];
    */

    let mut plans = vec![];
    let plan_len = 18 * num_rooms;
    for i in 0..plan_len {
        if false && rng.random_range(0..20) == 0 {
            plans.push(rng.random_range(0..6));
            continue;
        }

        let mut best = (OrderedFloat(0.0), OrderedFloat(0.0), !0);
        let mut order = (0..6).collect_vec();
        order.shuffle(&mut rng);
        for &d in &order {
            for d2 in 0..6 {
                let mut tmp_plans = plans.clone();
                tmp_plans.push(d);
                tmp_plans.push(d2);
                let evals = local_judges
                    .iter()
                    .map(|lj| coverage(lj, &tmp_plans))
                    .collect_vec();
                let tmp_coverage = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds as f32;
                let tmp_entropy = evals.iter().map(|(_, a, _)| a).sum::<f32>() / n_seeds as f32;

                best = best.max((OrderedFloat(tmp_coverage), OrderedFloat(tmp_entropy), d));
            }
        }
        eprintln!("Coverage {} {} {}", i, best.0, best.1);
        plans.push(best.2);
    }

    // 各数字の出てくる回数を表示
    let mut cnt = [0; 6];
    for &d in &plans {
        cnt[d] += 1;
    }
    eprintln!("Count: {}", cnt.iter().map(|&c| c.to_string()).join(" "));

    eprintln!("{}", plans.iter().map(|d| d.to_string()).join(""));

    plans
}

fn evaluate_plan(num_rooms: usize, plan: &Vec<usize>, seed_begin: usize, seed_end: usize) {
    let local_judges = (seed_begin..seed_end)
        .map(|i| LocalJudge::new("random", num_rooms, i as u64))
        .collect_vec();

    let evals = local_judges
        .iter()
        .map(|lj| coverage(lj, plan))
        .collect_vec();
    let coverage_avg =
        evals.iter().map(|(a, _, _)| a).sum::<f32>() / (seed_end - seed_begin) as f32;
    let entropy_avg = evals.iter().map(|(_, a, _)| a).sum::<f32>() / (seed_end - seed_begin) as f32;
    let perfect_avg = evals.iter().map(|(_, _, a)| a).sum::<f32>() / (seed_end - seed_begin) as f32;

    eprintln!(
        "Coverage: {:.6}, Entropy: {:.6}, Perfect: {:.6}",
        coverage_avg, entropy_avg, perfect_avg
    );
}

fn main() {
    let n_rooms = 30;
    let n_seeds = 1000;

    let plan = generate_plan(n_rooms, n_seeds);
    evaluate_plan(n_rooms, &plan, 0, n_seeds);
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);

    let plan = generate_plan_v2(n_rooms, n_seeds);
    evaluate_plan(n_rooms, &plan, 0, n_seeds);
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);

    // ランダムウォークを評価
    let mut rnd = rand::rng();
    let mut plan = vec![];
    for _ in 0..(n_rooms * 18) {
        let c: usize = rnd.random_range(0..6);
        plan.push(c);
    }
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);
}
