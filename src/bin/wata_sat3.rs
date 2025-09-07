#![allow(non_snake_case)]

use icfpc2025::{
    judge::Guess,
    solve_no_marks::{self, Cnf},
    *,
};
use itertools::Itertools;
use rand::prelude::*;

fn main() {
    let mut rng = rand::rng();
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let D = 3;
    let K = 1;
    let mut plans = vec![];
    let (super_guess, plans, labels) = match D {
        2 => {
            let mut plans0 = vec![vec![]; 2];
            for k in 0..2 {
                let mut plan = vec![];
                for _ in 0..judge.num_rooms() * 6 {
                    let door = rng.random_range(0..6);
                    plan.push((None, door));
                    plans0[k].push(door);
                }
                plans.push(plan);
            }
            for _ in 0..K {
                let mut plan = vec![];
                for _ in 0..judge.num_rooms() * 6 {
                    plan.push((Some(rng.random_range(0..4)), rng.random_range(0..6)));
                }
                plans.push(plan);
            }
            let mut labels = judge.explore(&plans);
            let labels0 = vec![labels[0].clone(), labels[1].clone()];
            let super_guess = solve_no_marks::solve(judge.num_rooms() / D, &plans0, &labels0);
            labels.remove(0);
            plans.remove(0);
            labels.remove(0);
            plans.remove(0);
            let mut flat_plans = vec![];
            let flat_labels = labels.iter().flatten().copied().collect_vec();
            for i in 0..plans.len() {
                flat_plans.extend(plans[i].iter().copied());
                if i + 1 < plans.len() {
                    flat_plans.push((None, !0));
                }
            }
            (super_guess, flat_plans, flat_labels)
        }
        3 => {
            let mut plan = vec![];
            let mut plans0 = vec![vec![]];
            for _ in 0..judge.num_rooms() * 6 {
                let door = rng.random_range(0..6);
                plan.push((None, door));
                plans0[0].push(door);
            }
            plans.push(plan);
            for _ in 0..K {
                plan = vec![];
                for _ in 0..judge.num_rooms() * 6 {
                    plan.push((Some(rng.random_range(0..4)), rng.random_range(0..6)));
                }
                plans.push(plan);
            }
            let mut labels = judge.explore(&plans);
            let labels0 = vec![labels[0].clone()];
            let super_guess = solve_no_marks::solve(judge.num_rooms() / D, &plans0, &labels0);
            labels.remove(0);
            plans.remove(0);
            let mut flat_plans = vec![];
            let flat_labels = labels.iter().flatten().copied().collect_vec();
            for i in 0..plans.len() {
                flat_plans.extend(plans[i].iter().copied());
                if i + 1 < plans.len() {
                    flat_plans.push((None, !0));
                }
            }
            (super_guess, flat_plans, flat_labels)
        }
        _ => panic!("not supported D"),
    };
    assert_eq!(plans.len() + 1, labels.len());
    let mut cnf = Cnf::new();
    let n = judge.num_rooms() / D;
    assert_eq!(super_guess.rooms.len(), n);
    // V[t][i] := 時刻 t に訪れたのは (u,i) である
    let mut V = mat![!0; labels.len(); D];
    for t in 0..labels.len() {
        for d in 0..D {
            V[t][d] = cnf.var();
        }
        cnf.choose_one(&V[t]);
    }
    // E[u][e][i][j] := u の e 番目のドアが (u,i) と (v,j) を結ぶ
    let mut E = mat![!0; n; 6; D; D];
    for u in 0..n {
        for e in 0..6 {
            for i in 0..D {
                for j in 0..D {
                    let (v, f) = super_guess.graph[u][e];
                    if (u, e, i, j) <= (v, f, j, i) {
                        E[u][e][i][j] = cnf.var();
                    } else {
                        E[u][e][i][j] = E[v][f][j][i];
                    }
                }
                cnf.choose_one(&E[u][e][i]);
            }
        }
    }
    let mut u = super_guess.start;
    cnf.clause([V[0][0]]);
    for t in 0..plans.len() {
        if plans[t].1 == !0 {
            u = super_guess.start;
            cnf.clause([V[t + 1][0]]);
        } else {
            let (_, e) = plans[t];
            let v = super_guess.graph[u][e].0;
            for i in 0..D {
                for j in 0..D {
                    // V[t][i] & E[u][e][i][j] -> V[t+1][j]
                    cnf.clause([-V[t][i], -E[u][e][i][j], V[t + 1][j]]);
                }
            }
            u = v;
        }
    }
    u = super_guess.start;
    // prev_t[u] := u に最後に訪れた時刻
    let mut prev_t = vec![!0; n];
    // C[t][i][c] := 時刻 t の終了時点で、(u,i) の色が c である
    let mut C = mat![!0; plans.len(); D; 4];
    for t in 0..plans.len() {
        for d in 0..D {
            for c in 0..4 {
                C[t][d][c] = cnf.var();
            }
            cnf.choose_one(&C[t][d]);
        }
        let v = if plans[t].1 == !0 {
            super_guess.start
        } else {
            super_guess.graph[u][plans[t].1].0
        };
        if let Some(new_c) = plans[t].0 {
            let pt = prev_t[u];
            if pt == !0 {
                assert_eq!(labels[t], super_guess.rooms[u], "graph inconsistent");
                for i in 0..D {
                    // V[t][i] -> C[t][i][new_c]
                    cnf.clause([-V[t][i], C[t][i][new_c]]);
                    // !V[t][i] -> C[t][i][labels[t]]
                    cnf.clause([V[t][i], C[t][i][labels[t]]]);
                }
            } else {
                for i in 0..D {
                    // V[t][i] -> C[pt][i][labels[t]] & C[t][i][new_c]
                    cnf.clause([-V[t][i], C[pt][i][labels[t]]]);
                    cnf.clause([-V[t][i], C[t][i][new_c]]);
                    for c in 0..4 {
                        // C[pt][i][c] & !V[t][i] -> C[t][i][c]
                        cnf.clause([-C[pt][i][c], V[t][i], C[t][i][c]]);
                    }
                }
            }
        } else {
            let pt = prev_t[u];
            if pt == !0 {
                assert_eq!(labels[t], super_guess.rooms[u]);
                for i in 0..D {
                    cnf.clause([C[t][i][labels[t]]]);
                }
            } else {
                for i in 0..D {
                    // V[t][i] -> C[pt][i][labels[t]]
                    cnf.clause([-V[t][i], C[pt][i][labels[t]]]);
                    for c in 0..4 {
                        // C[pt][i][c] -> C[t][i][c]
                        cnf.clause([-C[pt][i][c], C[t][i][c]]);
                    }
                }
            }
        }
        prev_t[u] = t;
        if plans[t].1 == !0 {
            prev_t.fill(!0);
        }
        u = v;
    }
    assert_eq!(cnf.sat.solve(), Some(true));
    let mut guess = Guess {
        start: super_guess.start * D,
        graph: vec![[(0, 0); 6]; judge.num_rooms()],
        rooms: vec![0; judge.num_rooms()],
    };
    for u in 0..n {
        for i in 0..D {
            guess.rooms[u * D + i] = super_guess.rooms[u];
        }
    }
    for u in 0..n {
        for e in 0..6 {
            let (v, f) = super_guess.graph[u][e];
            for i in 0..D {
                for j in 0..D {
                    if cnf.sat.value(E[u][e][i][j]) == Some(true) {
                        guess.graph[u * D + i][e] = (v * D + j, f);
                    }
                }
            }
        }
    }
    judge.guess(&guess);
}
