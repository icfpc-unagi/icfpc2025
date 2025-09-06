#![allow(
    clippy::needless_range_loop,
    clippy::useless_vec,
    clippy::partialeq_to_none,
    non_snake_case,
    unused_variables
)]
use icfpc2025::judge::*;

// SAT variable encoding functions.
// These functions map high-level concepts (like room visits, room properties, and edges)
// into unique integer variables for the SAT solver.

/// SAT variable encoding: the i-th room visited in the path is room u.
/// i: path index [0, q)
/// u: room index [0, n)
fn V(n: usize, q: usize, i: usize, u: usize) -> i32 {
    // i番目の頂点が頂点u
    // i: [0, q), u:[0, n)
    (1 + (i * n) + u) as i32
}

/// SAT variable encoding: room u has level (property) k.
/// u: room index [0, n)
/// k: level index [0, 3]
fn L(n: usize, q: usize, u: usize, k: usize) -> i32 {
    // u: [0, n), k: [0, 3)
    (1 + n * q + (u * 4) + k) as i32
}

/// SAT variable encoding: there is a connection from room u's door e to room v's door f.
/// u, v: room indices [0, n)
/// e, f: door indices [0, 5]
fn E(n: usize, q: usize, u: usize, e: usize, v: usize, f: usize) -> i32 {
    // u: [0, n), e: [0, 6), v: [0, n), f: [0, 6)
    (1 + n * q + (n * 4) + (u * 6 * n * 6) + (e * n * 6) + v * 6 + f) as i32
}

fn main() {
    let judge = get_judge_from_stdin_with(true);
    let n = judge.num_rooms();

    // Use pre-recorded explores instead of generating random route
    let explored = judge.explored();
    assert!(
        !explored.plans.is_empty(),
        "explored is empty; provide explores via JSON"
    );
    let plan = explored.plans[0].clone();
    let r = vec![explored.results[0].clone()];

    assert_eq!(r.len(), 1);
    let seq = &r[0];
    let q = seq.len();

    // Assertions to ensure the variable ranges do not overlap.
    assert_eq!(V(n, q, 0, 0), 1);
    assert_eq!(V(n, q, q - 1, n - 1) + 1, L(n, q, 0, 0));
    assert_eq!(L(n, q, n - 1, 3) + 1, E(n, q, 0, 0, 0, 0));

    // Initialize the SAT solver.
    // let mut sat: cadical::Solver = Default::default();
    let mut sat: cadical::Solver = cadical::Solver::with_config("sat").unwrap();

    // Add constraints to the SAT solver.

    // Path constraints: At each step i, we must be in exactly one room u.
    for i in 0..q {
        // At least one room.
        sat.add_clause((0..n).map(|u| V(n, q, i, u)));
        // At most one room.
        for u in 0..n {
            for v in (u + 1)..n {
                sat.add_clause([-V(n, q, i, u), -V(n, q, i, v)]);
            }
        }
    }

    // Room level constraints: Each room u must have exactly one level k.
    for u in 0..n {
        // At least one level.
        sat.add_clause((0..4).map(|k| L(n, q, u, k)));
        // At most one level.
        for k in 0..4 {
            for l in (k + 1)..4 {
                sat.add_clause([-L(n, q, u, k), -L(n, q, u, l)]);
            }
        }
    }

    // Observation constraints: If the i-th visited room is u, its level must match the observation seq[i].
    for i in 0..q {
        for u in 0..n {
            // もしi番目の頂点がuならば、uのレベルはseq[i]
            // V(n, q, i, u) => L(n, q, u, seq[i])
            sat.add_clause([-V(n, q, i, u), L(n, q, u, seq[i])]);
        }
    }

    // Transition constraints: If we move from room u to v via port e, an edge must exist.
    for i in 0..(q - 1) {
        let e = plan[i];
        for u in 0..n {
            for v in 0..n {
                // (V(i, u) AND V(i+1, v)) => exists f, E(u, e, v, f)
                sat.add_clause([
                    -V(n, q, i, u),
                    -V(n, q, i + 1, v),
                    E(n, q, u, e, v, 0),
                    E(n, q, u, e, v, 1),
                    E(n, q, u, e, v, 2),
                    E(n, q, u, e, v, 3),
                    E(n, q, u, e, v, 4),
                    E(n, q, u, e, v, 5),
                ]);
            }
        }
    }

    // 辺の行き先は一意であること
    // Edge uniqueness: Each port (u, e) must connect to exactly one destination (v, f).
    for u in 0..n {
        for e in 0..6 {
            // At least one connection.
            sat.add_clause((0..n).flat_map(|v| (0..6).map(move |f| E(n, q, u, e, v, f))));
            // At most one connection.
            for v in 0..n {
                for f in 0..6 {
                    for w in 0..n {
                        for g in 0..6 {
                            if (v, f) < (w, g) {
                                sat.add_clause([-E(n, q, u, e, v, f), -E(n, q, u, e, w, g)]);
                            }
                        }
                    }
                }
            }
        }
    }

    // undirectionalであること
    // Undirected edge constraint: If edge (u, e) -> (v, f) exists, then (v, f) -> (u, e) must also exist.
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    // E(u, e, v, f) <=> E(v, f, u, e)
                    sat.add_clause([-E(n, q, u, e, v, f), E(n, q, v, f, u, e)]);
                    sat.add_clause([E(n, q, u, e, v, f), -E(n, q, v, f, u, e)]);
                }
            }
        }
    }

    // Solve the SAT problem.
    assert_eq!(sat.solve(), Some(true));

    // --- Decoding the solution from the SAT model ---

    // rooms
    let mut rooms = vec![0; n];
    for u in 0..n {
        for k in 0..4 {
            let val = sat.value(L(n, q, u, k));
            if val == None {
                panic!();
            }
            if val == Some(true) {
                rooms[u] = k;
                break;
            }
        }
    }

    // starting room
    let mut start = None;
    for u in 0..n {
        let val = sat.value(V(n, q, 0, u));
        if val == None {
            panic!();
        }
        if val == Some(true) {
            start = Some(u);
            break;
        }
    }

    // graph (edges)
    let mut graph = vec![[(0, 0); 6]; n];
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    let val = sat.value(E(n, q, u, e, v, f));
                    if val == None {
                        panic!();
                    }
                    if val == Some(true) {
                        graph[u][e] = (v, f);
                    }
                }
            }
        }
    }

    dbg!(&graph);

    // Submit the decoded guess to the judge.
    judge.guess(&Guess {
        start: start.unwrap(),
        rooms,
        graph,
    });
}
