#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::judge::{JsonIn, *};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::env;
use std::io::Read;

type Edge = ((usize, usize), (usize, usize));

struct Instance {
    num_rooms: usize,
    // adjacency: next room for (room, door)
    graph: Vec<[usize; 6]>,
    // mapping from (room, door) to undirected edge id
    port_to_edge: Vec<[usize; 6]>,
    edge_count: usize,
}

fn build_instance(num_rooms: usize, edges: &[Edge]) -> Instance {
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
fn coverage(inst: &Instance, plan: &[usize]) -> (f32, f32, f32) {
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

fn shuffled_instances(
    num_rooms: usize,
    n_seeds: usize,
    base_seed: u64,
    base_edges: &[Edge],
) -> Vec<Instance> {
    let mut instances = Vec::with_capacity(n_seeds);
    for i in 0..n_seeds {
        let edges: Vec<Edge> = base_edges.to_vec();
        // Per-room door shuffle seeded from base_seed + i
        let mut rng = ChaCha20Rng::seed_from_u64(base_seed.wrapping_add(i as u64));
        let mut door_maps: Vec<[usize; 6]> = Vec::with_capacity(num_rooms);
        for _ in 0..num_rooms {
            let mut m = [0usize; 6];
            for (d, slot) in m.iter_mut().enumerate() {
                *slot = d;
            }
            m.shuffle(&mut rng);
            door_maps.push(m);
        }
        let mut remapped = Vec::with_capacity(edges.len());
        for &((u1, d1), (u2, d2)) in &edges {
            let nd1 = door_maps[u1][d1];
            let nd2 = door_maps[u2][d2];
            remapped.push(((u1, nd1), (u2, nd2)));
        }
        instances.push(build_instance(num_rooms, &remapped));
    }
    instances
}

fn read_seed_from_env() -> u64 {
    env::var("SEED")
        .ok()
        .and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<u64>().ok()
            }
        })
        .unwrap_or(0)
}

fn generate_plan(num_rooms: usize, n_seeds: usize, base_edges: &[Edge]) -> Vec<usize> {
    let mut rng = rand::rng();

    let base_seed = read_seed_from_env();
    let instances = shuffled_instances(num_rooms, n_seeds, base_seed, base_edges);

    /*
        let mut vis = vec![vec![[0; 6]; num_rooms]; n_seeds];
        let mut pos = vec![0; n_seeds];
    */

    let mut plans = vec![];
    let plan_len = 18 * num_rooms;
    for i in 0..plan_len {
        let mut best = (OrderedFloat(0.0), OrderedFloat(0.0), OrderedFloat(0.0), !0);
        let mut order = (0..6).collect_vec();
        order.shuffle(&mut rng);
        for &d in &order {
            if (i >= 1 && d == plans[i - 1]) || (i >= 2 && d == plans[i - 2]) {
                // tos sensei heuristic
                // continue;
            }

            let mut tmp_plans = plans.clone();
            tmp_plans.push(d);
            let evals = instances
                .iter()
                .map(|inst| coverage(inst, &tmp_plans))
                .collect_vec();
            // Optimize primarily for directed coverage (second element), then entropy
            let tmp_cov_und = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds as f32;
            let tmp_cov_dir = evals.iter().map(|(_, b, _)| b).sum::<f32>() / n_seeds as f32;
            let tmp_entropy = evals.iter().map(|(_, _, c)| c).sum::<f32>() / n_seeds as f32;

            best = best.max((
                OrderedFloat(tmp_cov_und),
                OrderedFloat(tmp_cov_dir),
                OrderedFloat(tmp_entropy),
                d,
            ));
        }
        eprintln!(
            "Step {} best: cov_und={} cov_dir={} entropy={}",
            i, best.0, best.1, best.2
        );
        plans.push(best.3);
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

fn generate_plan_v2(num_rooms: usize, n_seeds: usize, base_edges: &[Edge]) -> Vec<usize> {
    let mut rng = rand::rng();

    let base_seed = read_seed_from_env();
    let instances = shuffled_instances(num_rooms, n_seeds, base_seed, base_edges);

    /*
        let mut vis = vec![vec![[0; 6]; num_rooms]; n_seeds];
        let mut pos = vec![0; n_seeds];
    */

    let mut plans = vec![];
    let plan_len = 18 * num_rooms;
    for i in 0..plan_len {
        let mut best = (OrderedFloat(0.0), OrderedFloat(0.0), OrderedFloat(0.0), !0);
        let mut order = (0..6).collect_vec();
        order.shuffle(&mut rng);
        for &d in &order {
            for d2 in 0..6 {
                let mut tmp_plans = plans.clone();
                tmp_plans.push(d);
                tmp_plans.push(d2);
                let evals = instances
                    .iter()
                    .map(|inst| coverage(inst, &tmp_plans))
                    .collect_vec();
                let tmp_cov_und = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds as f32;
                let tmp_cov_dir = evals.iter().map(|(_, b, _)| b).sum::<f32>() / n_seeds as f32;
                let tmp_entropy = evals.iter().map(|(_, _, c)| c).sum::<f32>() / n_seeds as f32;

                best = best.max((
                    OrderedFloat(tmp_cov_und),
                    OrderedFloat(tmp_cov_dir),
                    OrderedFloat(tmp_entropy),
                    d,
                ));
            }
        }
        eprintln!(
            "Step {} best: cov_und={} cov_dir={} entropy={}",
            i, best.0, best.1, best.2
        );
        plans.push(best.3);
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

fn evaluate_plan(num_rooms: usize, plan: &[usize], seed_begin: usize, seed_end: usize) {
    let instances = (seed_begin..seed_end)
        .map(|i| {
            let edges = generate_random_edges_v2(num_rooms, i as u64);
            build_instance(num_rooms, &edges)
        })
        .collect_vec();

    let evals = instances
        .iter()
        .map(|inst| coverage(inst, plan))
        .collect_vec();
    let cov_undir_avg =
        evals.iter().map(|(a, _, _)| a).sum::<f32>() / (seed_end - seed_begin) as f32;
    let cov_dir_avg = evals.iter().map(|(_, b, _)| b).sum::<f32>() / (seed_end - seed_begin) as f32;
    let entropy_avg = evals.iter().map(|(_, _, c)| c).sum::<f32>() / (seed_end - seed_begin) as f32;

    eprintln!(
        "Coverage undirected: {:.6}, directed: {:.6}, Entropy: {:.6}",
        cov_undir_avg, cov_dir_avg, entropy_avg
    );
}

fn main() {
    // Accept same JSON format: may contain map, plans/results, numRooms
    let mut stdin_buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut stdin_buf);
    let stdin_trim = stdin_buf.trim();

    let (n_rooms, base_edges): (usize, Vec<Edge>) = if stdin_trim.starts_with('{') {
        let parsed: JsonIn = serde_json::from_str(stdin_trim).expect("invalid JSON for v3");
        if let Some(map) = parsed.map {
            let n = map.rooms.len();
            let mut edges: Vec<Edge> = Vec::with_capacity(n * 3);
            for c in map.connections.iter() {
                let fr = &c.from;
                let to = &c.to;
                if fr.room < n && fr.door < 6 && to.room < n && to.door < 6 {
                    edges.push(((fr.room, fr.door), (to.room, to.door)));
                }
            }
            (n, edges)
        } else {
            panic!("JSON must contain 'map' for v3 routing instances");
        }
    } else {
        panic!("Expected JSON input with 'map' for v3 routing instances");
    };

    let n_seeds = 10_000;

    let plan = generate_plan(n_rooms, n_seeds, &base_edges);
    evaluate_plan(n_rooms, &plan, 0, n_seeds);
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);

    let plan = generate_plan_v2(n_rooms, n_seeds, &base_edges);
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

    let plan: Vec<usize> = "413022551403315200442351124530532105441250342013450431221500235401332245541100524301142035531432234405215311350240015425104334025123304521120534103522445503114230032542135431452051002415012340523321554433021451132041025533014400351220440115324321342250451305502315412031045522433500142534115203442231104350310540221435132441500553020145133425523012412033443004231254411023052214500135410325345320121545042102113352405514315340154304422003355523411201445331322002514054225105033004323315550112134421441352532400511024124025405311304432221235".chars().map(|c| c.to_digit(10).unwrap() as usize).collect();
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);
}
