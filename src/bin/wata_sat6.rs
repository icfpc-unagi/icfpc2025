#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]

use icfpc2025::{judge::Guess, solve_no_marks::Cnf, *};
use itertools::Itertools;
use rand::prelude::*;

fn balanced_plan(len: usize, m: usize, rng: &mut impl Rng) -> Vec<usize> {
    let mut plan = Vec::with_capacity(len);
    for d in 0..len {
        plan.push(d % m);
    }
    plan.shuffle(rng);
    plan
}

fn gacha(n: usize, plan: &[(Option<usize>, usize)], labels: &[usize]) -> f64 {
    let mut label_door = mat![0; 4; 6];
    for i in 0..labels.len() {
        let door = plan[i].1;
        if door == !0 {
            continue;
        }
        let label = labels[i];
        label_door[label][door] += 1;
    }
    let mut sum = 0.0;
    let mut num = vec![0; 4];
    for i in 0..n {
        num[i % 4] += 1;
    }
    for i in 0..4 {
        for j in 0..6 {
            let expected = num[i] as f64 / n as f64 / 6.0;
            sum += (expected - label_door[i][j] as f64 / labels.len() as f64).powi(2);
        }
    }
    dbg!(sum);
    sum
}

fn main() {
    let mut rng = rand::rng();
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let K = 2;
    let F = judge.num_rooms() * 6; // 色を塗らずに動く回数
    let n = judge.num_rooms() / 2;
    let (plans, labels) = {
        let mut plans = vec![];
        let mut first = 0;
        for k in 0..K {
            let tmp = balanced_plan(judge.num_rooms() * 6, 6, &mut rng);
            plans.push(tmp.iter().map(|&d| (None, d)).collect_vec());
            if first + judge.num_rooms() * 6 <= F {
                first += judge.num_rooms() * 6;
            } else {
                let f = F - first;
                first += f;
                let mut b = balanced_plan(judge.num_rooms() * 6 - f, 4, &mut rng);
                for p in f..judge.num_rooms() * 6 {
                    plans[k][p].0 = b.pop();
                }
            }
        }
        let labels = judge.explore(&plans);
        let mut flat_plans = vec![];
        let flat_labels = labels.iter().flatten().copied().collect_vec();
        for i in 0..plans.len() {
            flat_plans.extend(plans[i].iter().copied());
            if i + 1 < plans.len() {
                flat_plans.push((None, !0));
            }
        }
        (flat_plans, flat_labels)
    };
    assert_eq!(plans.len() + 1, labels.len());
    let mut L = vec![0; n];
    for i in 0..n {
        L[i] = i % 4;
    }
    L.sort();
    if gacha(n, &plans, &labels[..=F]) > 0.0015 {
        panic!("unlucky");
    }
    let mut cnf = Cnf::new();

    // V[t][u] := 時刻 t の開始時点での頂点は u
    let mut V = mat![!0; labels.len(); n];
    for t in 0..labels.len() {
        for u in 0..n {
            V[t][u] = cnf.var();
        }
        cnf.choose_one(&V[t]);
    }
    let s = (0..n).find(|&u| labels[0] == L[u]).unwrap();
    cnf.clause([V[0][s]]);

    // A[u][e][v] := u の ドア e は v とつながる
    let mut A = mat![!0; n; 6; n];
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                A[u][e][v] = cnf.var();
            }
            cnf.choose_one(&A[u][e]);
        }
    }
    for t in 0..plans.len() {
        let e = plans[t].1;
        if e == !0 {
            cnf.clause([V[t + 1][s]]);
        } else {
            for u in 0..n {
                for v in 0..n {
                    // V[t][u] & A[u][e][v] -> V[t+1][v]
                    cnf.clause([-V[t][u], -A[u][e][v], V[t + 1][v]]);
                }
            }
        }
    }

    // E[u][e][v][f] := u のドア e は v のドア f とつながる
    let mut E = mat![!0; n; 6; n; 6];
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    if E[u][e][v][f] == !0 {
                        E[u][e][v][f] = cnf.var();
                        E[v][f][u][e] = E[u][e][v][f];
                    }
                    cnf.clause([-E[u][e][v][f], A[u][e][v]]);
                }
                cnf.amo_sequential(&E[u][e][v]);
                // A[u][e][v] -> OR(E[u][e][v][*])
                let mut tmp = E[u][e][v].clone();
                tmp.push(-A[u][e][v]);
                cnf.clause(tmp);
            }
        }
    }

    // F[u][e] := u の ドア e は状態反転
    let mut F = mat![!0; n; 6];
    for u in 0..n {
        for e in 0..6 {
            F[u][e] = cnf.var();
        }
    }
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    // F[u][e] & E[u][e][v][f] -> F[v][f]
                    cnf.clause([-F[u][e], -E[u][e][v][f], F[v][f]]);
                    // !F[u][e] & E[u][e][v][f] -> !F[v][f]
                    cnf.clause([F[u][e], -E[u][e][v][f], -F[v][f]]);
                }
            }
        }
    }

    // S[t] := 時刻 t の開始時点での状態
    let mut S = vec![!0; labels.len()];
    for t in 0..labels.len() {
        S[t] = cnf.var();
    }
    cnf.clause([-S[0]]);
    for t in 0..plans.len() {
        let e = plans[t].1;
        if e == !0 {
            cnf.clause([-S[t + 1]]);
        } else {
            for u in 0..n {
                // S[t] & V[t][u] & F[u][e] -> !S[t+1]
                cnf.clause([-S[t], -V[t][u], -F[u][e], -S[t + 1]]);
                // S[t] & V[t][u] & !F[u][e] -> S[t+1]
                cnf.clause([-S[t], -V[t][u], F[u][e], S[t + 1]]);
                // !S[t] & V[t][u] & F[u][e] -> S[t+1]
                cnf.clause([S[t], -V[t][u], -F[u][e], S[t + 1]]);
                // !S[t] & V[t][u] & !F[u][e] -> !S[t+1]
                cnf.clause([S[t], -V[t][u], F[u][e], -S[t + 1]]);
            }
        }
    }

    // C[t][ui][c] := 時刻 t の開始時点での ui の色は c
    let mut C = mat![!0; labels.len(); n * 2; 4];
    for t in 0..labels.len() {
        for ui in 0..n * 2 {
            for c in 0..4 {
                C[t][ui][c] = cnf.var();
            }
            cnf.choose_one(&C[t][ui]);
        }
    }
    for ui in 0..n * 2 {
        cnf.clause([C[0][ui][L[ui / 2]]]);
    }
    for t in 0..labels.len() {
        for u in 0..n {
            // V[t][u] & !S[t] -> C[t][u0][labels[t]]
            cnf.clause([-V[t][u], S[t], C[t][u * 2][labels[t]]]);
            // V[t][u] & S[t] -> C[t][u1][labels[t]]
            cnf.clause([-V[t][u], -S[t], C[t][u * 2 + 1][labels[t]]]);
        }
    }
    for t in 0..plans.len() {
        if let Some(newc) = plans[t].0 {
            for u in 0..n {
                // V[t][u] & !S[t] -> C[t+1][u0][newc]
                cnf.clause([-V[t][u], S[t], C[t + 1][u * 2][newc]]);
                // V[t][u] & S[t] -> C[t+1][u1][newc]
                cnf.clause([-V[t][u], -S[t], C[t + 1][u * 2 + 1][newc]]);
                for c in 0..4 {
                    // V[t][u] & !S[t] & C[t][u1][c] -> C[t+1][u1][c]
                    cnf.clause([-V[t][u], S[t], -C[t][u * 2 + 1][c], C[t + 1][u * 2 + 1][c]]);
                    // V[t][u] & S[t] & C[t][u0][c] -> C[t+1][u0][c]
                    cnf.clause([-V[t][u], -S[t], -C[t][u * 2][c], C[t + 1][u * 2][c]]);
                    // !V[t][u] & C[t][u0][c] -> C[t+1][u0][c]
                    cnf.clause([V[t][u], -C[t][u * 2][c], C[t + 1][u * 2][c]]);
                    // !V[t][u] & C[t][u1][c] -> C[t+1][u1][c]
                    cnf.clause([V[t][u], -C[t][u * 2 + 1][c], C[t + 1][u * 2 + 1][c]]);
                }
            }
        } else {
            if plans[t].1 == !0 {
                for ui in 0..n * 2 {
                    cnf.clause([C[t + 1][ui][L[ui / 2]]]);
                }
            } else {
                for u in 0..n {
                    for c in 0..4 {
                        // C[t][u0][c] -> C[t+1][u0][c]
                        cnf.clause([-C[t][u * 2][c], C[t + 1][u * 2][c]]);
                        // C[t][u1][c] -> C[t+1][u1][c]
                        cnf.clause([-C[t][u * 2 + 1][c], C[t + 1][u * 2 + 1][c]]);
                    }
                }
            }
        }
    }

    assert_eq!(cnf.sat.solve(), Some(true));
    let mut guess = Guess {
        start: s * 2,
        graph: vec![[(!0, !0); 6]; judge.num_rooms()],
        rooms: vec![0; judge.num_rooms()],
    };
    for u in 0..n {
        for i in 0..2 {
            guess.rooms[u * 2 + i] = L[u];
        }
    }
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    if E[u][e][v][f] != !0 && cnf.sat.value(E[u][e][v][f]) == Some(true) {
                        assert!(guess.graph[u * 2][e] == (!0, !0));
                        assert!(cnf.sat.value(E[v][f][u][e]) == Some(true));
                        if cnf.sat.value(F[u][e]) == Some(true) {
                            guess.graph[u * 2][e] = (v * 2 + 1, f);
                            guess.graph[u * 2 + 1][e] = (v * 2, f);
                        } else {
                            guess.graph[u * 2][e] = (v * 2, f);
                            guess.graph[u * 2 + 1][e] = (v * 2 + 1, f);
                        }
                    }
                }
            }
        }
    }
    assert!(judge.guess(&guess));
}
