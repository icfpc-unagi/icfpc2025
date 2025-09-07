#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]
use icfpc2025::judge::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::prelude::*;

// copy vvvvvvvvvv
struct Instance {
    num_rooms: usize,
    // adjacency: next room for (room, door)
    graph: Vec<[usize; 6]>,
    // mapping from (room, door) to undirected edge id
    port_to_edge: Vec<[usize; 6]>,
    edge_count: usize,
}

fn build_instance(num_rooms: usize, edges: &Vec<((usize, usize), (usize, usize))>) -> Instance {
    let mut graph = vec![[!0usize; 6]; num_rooms];
    let mut port_to_edge = vec![[!0usize; 6]; num_rooms];
    for (eid, &((u1, d1), (u2, d2))) in edges.iter().enumerate() {
        graph[u1][d1] = u2;
        graph[u2][d2] = u1;
        port_to_edge[u1][d1] = eid;
        port_to_edge[u2][d2] = eid;
    }
    Instance {
        num_rooms,
        graph,
        port_to_edge,
        edge_count: edges.len(),
    }
}

// Returns: (ratio_covered_undirected, ratio_covered_directed, normalized_entropy)
fn coverage(inst: &Instance, plan: &Vec<usize>) -> (f32, f32, f32) {
    let n = inst.num_rooms;
    let mut cnt = vec![[0usize; 6]; n];
    let mut edge_covered = vec![false; inst.edge_count];
    let mut u = 0;

    for &d in plan {
        cnt[u][d] += 1;
        let eid = inst.port_to_edge[u][d];
        if eid != !0usize {
            edge_covered[eid] = true;
        }
        u = inst.graph[u][d];
    }

    // directed coverage: how many (room,door) visited at least once
    let total_directed_covered = cnt
        .iter()
        .map(|x| x.iter().filter(|&&b| b >= 1).count())
        .sum::<usize>();
    let ratio_covered_directed = total_directed_covered as f32 / (n * 6) as f32;

    // undirected coverage: how many undirected edges visited at least once in any direction
    let total_undirected_covered = edge_covered.iter().filter(|&&b| b).count();
    let ratio_covered_undirected = total_undirected_covered as f32 / inst.edge_count as f32;

    // entropy over door usage per room
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
        / (n as f32 * 6.0f32.log2());

    (
        ratio_covered_undirected,
        ratio_covered_directed,
        normalized_entropy,
    )
}
// copy ^^^^^^^^^^

fn evaluate(plan: &Vec<usize>, seed_bgn: u64, seed_end: u64) -> f32 {
    let n_rooms = plan.len() / 18;

    let instances = (seed_bgn..seed_end)
        .map(|i| {
            let edges = generate_random_edges_v2(n_rooms, i as u64);
            build_instance(n_rooms, &edges)
        })
        .collect_vec();

    let evals = instances
        .iter()
        .map(|inst| coverage(inst, &plan))
        .collect_vec();

    let n_seeds = (seed_end - seed_bgn) as f32;
    let tmp_cov_und = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds;
    let tmp_cov_dir = evals.iter().map(|(_, b, _)| b).sum::<f32>() / n_seeds;
    let tmp_entropy = evals.iter().map(|(_, _, c)| c).sum::<f32>() / n_seeds;

    return tmp_cov_und * 100.0 + tmp_cov_dir * 10.0 + tmp_entropy;

    /*
    let tmp_cov_und = (tmp_cov_und * 1000.0).round() / 1000.0;
    let tmp_cov_dir = (tmp_cov_dir * 1000.0).round() / 1000.0;
    let tmp_entropy = (tmp_entropy * 1000.0).round() / 1000.0;

    tmp_cov_und * 1_0000_0000.0 + tmp_cov_dir * 1_0000.0 + tmp_entropy
    */
}

fn neighbor(crr_plan: &Vec<usize>, rng: &mut impl Rng) -> Vec<usize> {
    let kind = rng.random_range(0..3);
    if kind == 0 {
        // Reverse
        let mut swap_lft;
        let mut swap_rgt;
        loop {
            swap_lft = rng.random_range(0..crr_plan.len());
            swap_rgt = rng.random_range(0..crr_plan.len());
            if swap_rgt >= swap_lft + 2 {
                break;
            }
        }
        let mut nxt_plan = crr_plan.clone();
        nxt_plan[swap_lft..swap_rgt].reverse();
        nxt_plan
    } else if kind == 1 {
        // Swap
        let i = rng.random_range(0..crr_plan.len() - 1);
        let mut nxt_plan = crr_plan.clone();
        nxt_plan.swap(i, i + 1);
        nxt_plan
    } else {
        // Change
        let mut nxt_plan = crr_plan.clone();
        let mut change_pos;
        loop {
            change_pos = rng.random_range(0..crr_plan.len());
            let new_door = rng.random_range(0..6);
            if new_door != nxt_plan[change_pos] {
                nxt_plan[change_pos] = new_door;
                break;
            }
        }
        nxt_plan
    }
}

fn hillclimb(mut crr_plan: Vec<usize>, n_seeds_train_batch: u64, n_seeds_test: u64) {
    let patience = 100;
    let test_seed_bgn = 1_000_000_000;
    let mut rng = rand::rng();

    let mut bst_plan = crr_plan.clone();
    let mut bst_score_test = evaluate(&bst_plan, test_seed_bgn, test_seed_bgn + n_seeds_test);
    let mut n_steps_no_improve = 0;

    for step in 0.. {
        let seed_bgn = n_seeds_train_batch * step;
        let seed_end = n_seeds_train_batch * (step + 1);

        let crr_score_train = evaluate(&crr_plan, seed_bgn, seed_end);
        let nxt_plan = neighbor(&crr_plan, &mut rng);
        // let nxt_plan = neighbor(&nxt_plan, &mut rng);
        let nxt_score_train = evaluate(&nxt_plan, seed_bgn, seed_end);
        let mut improved = false;

        eprintln!(
            "Step {} train --- nxt={} crr={}, no-improve={}",
            step, nxt_score_train, crr_score_train, n_steps_no_improve
        );

        if nxt_score_train > crr_score_train {
            // 山自体はtrainだけ見て登る
            crr_plan = nxt_plan.clone();

            // 出力するかどうかはbestとtestで対決。
            let nxt_score_test = evaluate(&crr_plan, test_seed_bgn, test_seed_bgn + n_seeds_test);

            eprintln!(
                " Step {} test --- nxt={} bst={} ({})",
                step,
                nxt_score_test,
                bst_score_test,
                nxt_score_test > bst_score_test
            );

            if nxt_score_test > bst_score_test {
                bst_plan = nxt_plan.clone();
                bst_score_test = nxt_score_test;

                eprintln!(
                    "Step {} test={} plan=\n{}\n\n",
                    step,
                    bst_score_test,
                    bst_plan.iter().map(|d| d.to_string()).join("")
                );

                improved = true;
            }
        }

        if !improved {
            n_steps_no_improve += 1;
        }
        if n_steps_no_improve >= patience {
            n_steps_no_improve = 0;
            crr_plan = bst_plan.clone();
            eprintln!("Restarting from best known solution...");
        }
    }
}

fn balanced_plan(n: usize) -> Vec<usize> {
    let mut rng = rand::rng();
    let len = 18 * n;
    let mut plan = Vec::with_capacity(len);
    for d in 0..6 {
        for _ in 0..(len / 6) {
            plan.push(d);
        }
    }
    plan.shuffle(&mut rng);
    plan
}

fn main() {
    let n_seeds_train = 10000;
    let n_seeds_test = 10000;

    let plan = "413022551403315200442351124530532105441250342013450431221500235401332245541100524301142035531432234405215311350240015425104334025123304521120534103522445503114230032542135431452051002415012340523321554433021451132041025533014400351220440115324321342250451305502315412031045522433500142534115203442231104350310540221435132441500553020145133425523012412033443004231254411023052214500135410325345320121545042102113352405514315340154304422003355523411201445331322002514054225105033004323315550112134421441352532400511024124025405311304432221235";
    let plan = plan
        .chars()
        .map(|c| c.to_digit(10).unwrap() as usize)
        .collect::<Vec<_>>();

    // let plan = balanced_plan(30);

    hillclimb(plan, n_seeds_train, n_seeds_test);
}

// 424505152015335015143350055400341551123125553430404413111501020143452123024104104122233254013413101021201512221405411421041030022340445410313124303525014112221543430542321134002254231232510012212530113521352342502442032304035334011511420133320052530451431014500015534425540342252230524513303253130420503543042331521014253233511124013122444050224112152550424514354315530215043152522443322051044255034413244300243200333341441441052435535334153335525544022355100105155002542314052050401225431031145343400325455001204504351522032134055303244021
