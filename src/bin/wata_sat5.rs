#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case, dead_code)]

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

fn gacha(L: &[usize], labels: &[usize]) -> f64 {
    let mut expected = [0; 4];
    for &c in L {
        expected[c] += 1;
    }
    let mut actual = [0; 4];
    for &c in labels {
        actual[c] += 1;
    }
    let mut sum = 0.0;
    for c in 0..4 {
        let e = expected[c] as f64 / L.len() as f64;
        let a = actual[c] as f64 / labels.len() as f64;
        sum += (e - a) * (e - a);
    }
    dbg!(sum);
    sum
}

fn main() {
    let mut rng = rand::rng();
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let H = judge.num_rooms() * 2; // 色を塗らずに動く回数
    let n = judge.num_rooms() / 3;
    let mut plans = balanced_plan(judge.num_rooms() * 6, 6, &mut rng)
        .into_iter()
        .map(|e| (None, e))
        .collect_vec();
    let cs = balanced_plan(plans.len() - H, 4, &mut rng);
    for i in H..plans.len() {
        plans[i].0 = Some(cs[i - H]);
    }
    let labels = judge.explore(&[plans.clone()])[0].clone();
    assert_eq!(plans.len() + 1, labels.len());
    let mut L = vec![0; n];
    for i in 0..n {
        L[i] = i % 4;
    }
    L.sort();
    // if gacha(&L, &labels[..=H]) > 0.002 {
    //     panic!("unlucky");
    // }
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
        for u in 0..n {
            for v in 0..n {
                // V[t][u] & A[u][e][v] -> V[t+1][v]
                cnf.clause([-V[t][u], -A[u][e][v], V[t + 1][v]]);
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

    let perms = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [2, 1, 0],
        [1, 2, 0],
        [2, 0, 1],
    ];
    let perm_rev = [0, 1, 2, 3, 5, 4];
    for p in 0..6 {
        for k in 0..3 {
            assert_eq!(perms[perm_rev[p]][perms[p][k]], k);
        }
    }
    // P[u][e][p] := u の ドア e のPermutationが p である
    let mut P = mat![!0; n; 6; 6];
    for u in 0..n {
        for e in 0..6 {
            for p in 0..6 {
                P[u][e][p] = cnf.var();
            }
            cnf.choose_one(&P[u][e]);
        }
    }
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    for p in 0..6 {
                        // E[u][e][v][f] & P[u][e][p] -> P[v][f][perm_rev[p]]
                        cnf.clause([-E[u][e][v][f], -P[u][e][p], P[v][f][perm_rev[p]]]);
                    }
                }
            }
        }
    }

    // S[t][k] := 時刻 t の開始時点での状態が k
    let mut S = mat![!0; labels.len(); 3];
    for t in 0..labels.len() {
        for k in 0..3 {
            S[t][k] = cnf.var();
        }
        cnf.choose_one(&S[t]);
    }
    cnf.clause([-S[0][0]]);
    for t in 0..plans.len() {
        let e = plans[t].1;
        for u in 0..n {
            for k in 0..3 {
                for p in 0..6 {
                    // S[t][k] & V[t][u] & P[u][e][p] -> S[t+1][perms[p][k]]
                    cnf.clause([-S[t][k], -V[t][u], -P[u][e][p], S[t + 1][perms[p][k]]]);
                }
            }
        }
    }

    // C[t][ui][c] := 時刻 t の開始時点での ui の色は c
    let mut C = mat![!0; labels.len(); n * 3; 4];
    for t in 0..labels.len() {
        for ui in 0..n * 3 {
            for c in 0..4 {
                C[t][ui][c] = cnf.var();
            }
            cnf.choose_one(&C[t][ui]);
        }
    }
    for ui in 0..n * 3 {
        cnf.clause([C[0][ui][L[ui / 3]]]);
    }
    for t in 0..labels.len() {
        for u in 0..n {
            for k in 0..3 {
                let uk = u * 3 + k;
                // V[t][u] & S[t][k] -> C[t][uk][labels[t]]
                cnf.clause([-V[t][u], -S[t][k], C[t][uk][labels[t]]]);
            }
        }
    }
    for t in 0..plans.len() {
        if let Some(newc) = plans[t].0 {
            for u in 0..n {
                for k in 0..3 {
                    let uk = u * 3 + k;
                    // V[t][u] & S[t][k] -> C[t+1][uk][newc]
                    cnf.clause([-V[t][u], -S[t][k], C[t + 1][uk][newc]]);
                    for c in 0..4 {
                        // V[t][u] & !S[t][k] & C[t][uk][c] -> C[t+1][uk][c]
                        cnf.clause([-V[t][u], S[t][k], -C[t][uk][c], C[t + 1][uk][c]]);
                        // !V[t][u] & C[t][uk][c] -> C[t+1][uk][c]
                        cnf.clause([V[t][u], -C[t][uk][c], C[t + 1][uk][c]]);
                    }
                }
            }
        } else {
            for u in 0..n {
                for k in 0..3 {
                    let uk = u * 3 + k;
                    for c in 0..4 {
                        // V[t][u] & C[t][uk][c] -> C[t+1][uk][c]
                        cnf.clause([-V[t][u], -C[t][uk][c], C[t + 1][uk][c]]);
                    }
                }
            }
        }
    }

    assert_eq!(cnf.sat.solve(), Some(true));
    let mut guess = Guess {
        start: s * 3,
        graph: vec![[(!0, !0); 6]; judge.num_rooms()],
        rooms: vec![0; judge.num_rooms()],
    };
    for u in 0..n {
        for i in 0..3 {
            guess.rooms[u * 3 + i] = L[u];
        }
    }
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    if E[u][e][v][f] != !0 && cnf.sat.value(E[u][e][v][f]) == Some(true) {
                        assert!(guess.graph[u * 3][e] == (!0, !0));
                        assert!(cnf.sat.value(E[v][f][u][e]) == Some(true));
                        for p in 0..6 {
                            if cnf.sat.value(P[u][e][p]) == Some(true) {
                                for k in 0..3 {
                                    guess.graph[u * 3 + k][e] = (v * 3 + perms[p][k], f);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(judge.guess(&guess));
}
