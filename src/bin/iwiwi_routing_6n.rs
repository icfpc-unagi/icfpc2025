#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::judge::*;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use rand::prelude::*;

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
    let mut vertex_covered = vec![false; n];
    let mut edge_covered = vec![false; inst.edge_count];
    let mut u = 0;

    vertex_covered[0] = true;
    for &d in plan {
        cnt[u][d] += 1;
        let eid = inst.port_to_edge[u][d];
        if eid != !0usize {
            edge_covered[eid] = true;
        }
        u = inst.graph[u][d];
        vertex_covered[u] = true;
    }

    let total_vertex_covered = vertex_covered.iter().filter(|&&b| b).count();
    let ratio_covered_vertices = total_vertex_covered as f32 / n as f32;

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
        ratio_covered_vertices,
        ratio_covered_undirected,
        ratio_covered_directed,
    )
}

fn generate_plan_v2(num_rooms: usize, n_seeds: usize) -> Vec<usize> {
    let mut rng = rand::rng();

    let instances = (0..n_seeds)
        .map(|i| {
            let edges = generate_random_edges_v2(num_rooms, i as u64);
            build_instance(num_rooms, &edges)
        })
        .collect_vec();

    let mut plans = vec![];
    let plan_len = 6 * num_rooms;
    for i in 0..plan_len {
        if false && rng.random_range(0..20) == 0 {
            plans.push(rng.random_range(0..6));
            continue;
        }

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
                let tmp_cov_vtx = evals.iter().map(|(a, _, _)| a).sum::<f32>() / n_seeds as f32;
                let tmp_cov_uni = evals.iter().map(|(_, b, _)| b).sum::<f32>() / n_seeds as f32;
                let tmp_cov_dir = evals.iter().map(|(_, _, c)| c).sum::<f32>() / n_seeds as f32;

                best = best.max((
                    OrderedFloat(tmp_cov_vtx),
                    OrderedFloat(tmp_cov_uni),
                    OrderedFloat(tmp_cov_dir),
                    d,
                ));
            }
        }
        /*
        eprintln!(
            "Step {} best: cov_vtx={} cov_uni={} cov_dir={}",
            i, best.0, best.1, best.2
        );
        */
        plans.push(best.3);
    }

    // 各数字の出てくる回数を表示
    let mut cnt = [0; 6];
    for &d in &plans {
        cnt[d] += 1;
    }
    // eprintln!("Count: {}", cnt.iter().map(|&c| c.to_string()).join(" "));

    eprintln!("{}", plans.iter().map(|d| d.to_string()).join(""));

    plans
}

fn generate_plan_v3(num_rooms: usize, n_seeds: usize) -> Vec<usize> {
    let mut rng = rand::rng();

    // Prepare instances as in v2
    let instances = (0..n_seeds)
        .map(|i| {
            let edges = generate_random_edges_v2(num_rooms, i as u64);
            build_instance(num_rooms, &edges)
        })
        .collect_vec();

    // Per-instance incremental state
    struct InstState {
        cur: usize,
        vertex_visit: Vec<u32>,   // number of visits per vertex
        dir_visit: Vec<[u32; 6]>, // number of visits per (vertex, door)
        edge_visit: Vec<u32>,     // number of traversals per undirected edge
        covered_v: u32,           // # of vertices visited at least once
        covered_dir: u32,         // # of directed (vertex, door) visited at least once
        covered_edge: u32,        // # of undirected edges traversed at least once
    }

    let mut states: Vec<InstState> = instances
        .iter()
        .map(|inst| InstState {
            cur: 0,
            vertex_visit: {
                let mut v = vec![0u32; inst.num_rooms];
                v[0] = 1; // start at room 0
                v
            },
            dir_visit: vec![[0u32; 6]; inst.num_rooms],
            edge_visit: vec![0u32; inst.edge_count],
            covered_v: 1,
            covered_dir: 0,
            covered_edge: 0,
        })
        .collect();

    let mut plans = vec![];
    let plan_len = 6 * num_rooms;

    // Denominators for averages (same across all instances)
    let denom_vtx = (n_seeds as f32) * (num_rooms as f32);
    let edge_count = instances[0].edge_count as f32; // 3 * num_rooms
    let denom_uni = (n_seeds as f32) * edge_count;
    let denom_dir = (n_seeds as f32) * ((num_rooms * 6) as f32);

    for _ in 0..plan_len {
        let mut best = (
            OrderedFloat(0.0),
            OrderedFloat(0.0),
            OrderedFloat(0.0),
            !0usize,
        );

        // Randomize evaluation order of first moves like v2
        let mut order = (0..6).collect_vec();
        order.shuffle(&mut rng);

        for &d in &order {
            // Precompute first-step hypotheticals across all instances
            let mut v_after = vec![0usize; n_seeds];
            let mut e1 = vec![!0usize; n_seeds];
            let mut inc1_vtx = vec![0u32; n_seeds];
            let mut inc1_dir = vec![0u32; n_seeds];
            let mut inc1_uni = vec![0u32; n_seeds];

            // Base sums after first hypothetical step (previous + inc1)
            let mut base_v_sum: u32 = 0;
            let mut base_dir_sum: u32 = 0;
            let mut base_uni_sum: u32 = 0;

            for (j, (inst, st)) in instances.iter().zip(states.iter()).enumerate() {
                let u = st.cur;
                let v = inst.graph[u][d];
                v_after[j] = v;

                // Directed (u,d)
                let dir_new = (st.dir_visit[u][d] == 0) as u32;
                inc1_dir[j] = dir_new;

                // Undirected edge through (u,d)
                let eid = inst.port_to_edge[u][d];
                e1[j] = eid;
                let uni_new = if eid != !0usize && st.edge_visit[eid] == 0 {
                    1
                } else {
                    0
                };
                inc1_uni[j] = uni_new;

                // Vertex v
                let vtx_new = (st.vertex_visit[v] == 0) as u32;
                inc1_vtx[j] = vtx_new;

                base_v_sum += st.covered_v + vtx_new;
                base_dir_sum += st.covered_dir + dir_new;
                base_uni_sum += st.covered_edge + uni_new;
            }

            // Evaluate second step d2 for this first move d
            for d2 in 0..6 {
                let mut inc2_v_sum: u32 = 0;
                let mut inc2_dir_sum: u32 = 0;
                let mut inc2_uni_sum: u32 = 0;

                for (j, (inst, st)) in instances.iter().zip(states.iter()).enumerate() {
                    let u = st.cur;
                    let v = v_after[j];

                    // Directed at (v, d2)
                    let mut dir_was = st.dir_visit[v][d2] > 0;
                    if !dir_was && v == u && d2 == d && inc1_dir[j] == 1 {
                        // First step already visits (u, d)
                        dir_was = true;
                    }
                    let inc2_dir = (!dir_was) as u32;
                    inc2_dir_sum += inc2_dir;

                    // Undirected edge at e2 = (v, d2)
                    let e2 = inst.port_to_edge[v][d2];
                    let mut edge_was = e2 != !0usize && st.edge_visit[e2] > 0;
                    if !edge_was && e2 == e1[j] && inc1_uni[j] == 1 {
                        edge_was = true;
                    }
                    let inc2_uni = (!edge_was && e2 != !0usize) as u32;
                    inc2_uni_sum += inc2_uni;

                    // Vertex w after taking (v, d2)
                    let w = inst.graph[v][d2];
                    let mut vtx_was = st.vertex_visit[w] > 0;
                    if !vtx_was && w == v && inc1_vtx[j] == 1 {
                        vtx_was = true;
                    }
                    let inc2_v = (!vtx_was) as u32;
                    inc2_v_sum += inc2_v;
                }

                let cov_vtx = (base_v_sum + inc2_v_sum) as f32 / denom_vtx;
                let cov_uni = (base_uni_sum + inc2_uni_sum) as f32 / denom_uni;
                let cov_dir = (base_dir_sum + inc2_dir_sum) as f32 / denom_dir;

                best = best.max((
                    OrderedFloat(cov_vtx),
                    OrderedFloat(cov_uni),
                    OrderedFloat(cov_dir),
                    d,
                ));
            }
        }

        // Commit the chosen first move across all instances
        let chosen_d = best.3;
        plans.push(chosen_d);

        for (inst, st) in instances.iter().zip(states.iter_mut()) {
            let u = st.cur;

            // Directed (u, chosen_d)
            if st.dir_visit[u][chosen_d] == 0 {
                st.covered_dir += 1;
            }
            st.dir_visit[u][chosen_d] += 1;

            // Undirected edge
            let eid = inst.port_to_edge[u][chosen_d];
            if eid != !0usize {
                if st.edge_visit[eid] == 0 {
                    st.covered_edge += 1;
                }
                st.edge_visit[eid] += 1;
            }

            // Move to next vertex
            let v = inst.graph[u][chosen_d];
            if st.vertex_visit[v] == 0 {
                st.covered_v += 1;
            }
            st.vertex_visit[v] += 1;
            st.cur = v;
        }
    }

    // Same diagnostics as v2 (optional)
    let mut cnt = [0; 6];
    for &d in &plans {
        cnt[d] += 1;
    }
    // eprintln!("Count: {}", cnt.iter().map(|&c| c.to_string()).join(" "));

    eprintln!("{}", plans.iter().map(|d| d.to_string()).join(""));

    plans
}

fn evaluate_plan(num_rooms: usize, plan: &Vec<usize>, seed_begin: usize, seed_end: usize) {
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
    let cov_vtx_avg = evals.iter().map(|(a, _, _)| a).sum::<f32>() / (seed_end - seed_begin) as f32;
    let cov_uni_avg = evals.iter().map(|(_, b, _)| b).sum::<f32>() / (seed_end - seed_begin) as f32;
    let cov_dir_avg = evals.iter().map(|(_, _, c)| c).sum::<f32>() / (seed_end - seed_begin) as f32;

    eprintln!(
        "Len: {} | Coverage vertex: {:.6}, undirected: {:.6}, directed: {:.6}",
        plan.len(),
        cov_vtx_avg,
        cov_uni_avg,
        cov_dir_avg
    );
}

fn doit(n_rooms: usize) -> Vec<usize> {
    let n_seeds = 100000;

    // let plan = generate_plan_v2(n_rooms, n_seeds);
    // evaluate_plan(n_rooms, &plan, 0, n_seeds);
    // evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);

    let plan = generate_plan_v3(n_rooms, n_seeds);
    evaluate_plan(n_rooms, &plan, 0, n_seeds);
    evaluate_plan(n_rooms, &plan, n_seeds, n_seeds * 2);

    // ランダムウォークを評価
    let mut rnd = rand::rng();
    let mut plan_random = vec![];
    for _ in 0..(n_rooms * 6) {
        let c: usize = rnd.random_range(0..6);
        plan_random.push(c);
    }
    evaluate_plan(n_rooms, &plan_random, n_seeds, n_seeds * 2);

    plan
}

fn main() {
    let sizes = [12, 24, 36, 48, 60, 18, 36, 54, 72, 90];
    let mut size_to_plan = std::collections::HashMap::new();
    for &size in &sizes {
        let plan = doit(size);
        size_to_plan.insert(size, plan);
    }

    println!("match n_rooms {{");
    for (size, plan) in size_to_plan {
        println!(
            "    {} => vec![{}],",
            size,
            plan.iter().map(|d| d.to_string()).join(", ")
        );
    }
    println!("    _ => panic!(),");
    println!("}}");
}
