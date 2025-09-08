#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]

use icfpc2025::{
    judge::Guess,
    solve_no_marks::{self, Cnf},
    *,
};
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
    for i in 0..labels.len().min(plan.len()) {
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
    let D = 3; // 倍化率
    let K = 2; // 全体のクエリ数
    let F = judge.num_rooms() * 6; // 前半パートの長さ
    let n = judge.num_rooms() / D;
    let (super_guess, plans, labels) = {
        let mut plans = vec![
            "053421124355003145223044132102540153351203445023114200554324125133051042215014033152443520411325530244002234511032054154230134552103501221433402532514310044152332500144551240530123153410521354220330420524115043021334514011522400543355322502431104320154423513402104531230554420011342541350314220511225053310324405552341300214450322545125330150043123141012421453202513005434045013322443102352331551412002403415510035111204255404452032".chars().map(|c| (None, (c as u8 - b'0') as usize)).collect_vec(),
            "143253012045123154024035210344105335022134504215312305401425520043513415324110233205542011352443021450052331041225035144203155430112405132245340015225143103225445300211441300423553243215044013305522034125103342540511502433410452314003213542205114255304115205435013312004455223451002355142102533012413420431155003422452154314053320142301324355410450122132005441233502115340254403110522310024155120330453251250443523112052432105445033".chars().map(|c| (None, (c as u8 - b'0') as usize)).collect_vec(),
        ];
        let tmp = balanced_plan(judge.num_rooms() * 6, 4, &mut rng);
        for i in 0..plans[1].len() {
            plans[1][i].0 = Some(tmp[i]);
        }
        let plans0 = vec![plans[0].iter().map(|a| a.1).collect_vec()];
        let mut labels = judge.explore(&plans);
        if gacha(n, &plans[0], &labels[0]) > 0.0015 {
            panic!("unlucky");
        }
        let mut labels0 = vec![];
        let mut first = 0;
        for k in 0..K {
            if first + judge.num_rooms() * 6 <= F {
                labels0.push(labels[k].clone());
                first += judge.num_rooms() * 6;
            } else {
                let f = F - first;
                first += f;
                if f > 0 {
                    labels0.push(labels[k][..f + 1].to_vec());
                }
            }
        }
        // let super_guess = solve_no_marks::solve(judge.num_rooms() / D, &plans0, &labels0);
        let super_guess =
            solve_no_marks::solve_cadical_multi(judge.num_rooms() / D, &plans0, &labels0, 50);
        eprintln!("!!!! super_guess done");
        while plans[0].iter().all(|x| x.0.is_none()) {
            plans.remove(0);
            labels.remove(0);
        }
        let mut flat_plans = vec![];
        let flat_labels = labels.iter().flatten().copied().collect_vec();
        for i in 0..plans.len() {
            flat_plans.extend(plans[i].iter().copied());
            if i + 1 < plans.len() {
                flat_plans.push((None, !0));
            }
        }
        (super_guess, flat_plans, flat_labels)
    };
    assert_eq!(plans.len() + 1, labels.len());
    let mut cnf = Cnf::new();
    assert_eq!(super_guess.rooms.len(), n);
    // V[t][i] := 時刻 t に訪れたのは (u,i) である
    let mut V = mat![!0; labels.len(); D];
    for t in 0..labels.len() {
        for d in 0..D {
            V[t][d] = cnf.var();
        }
        cnf.choose_one(&V[t]);
    }
    // E[u'][e][v'][f] := u' の e 番目のドアが v' の f 番目 を結ぶ
    let mut E = mat![!0; n * D; 6; n * D; 6];
    for u in 0..n {
        for e in 0..6 {
            let (v, _) = super_guess.graph[u][e];
            for f in 0..6 {
                if super_guess.graph[v][f].0 == u {
                    for i in 0..D {
                        for j in 0..D {
                            let ui = u * D + i;
                            let vj = v * D + j;
                            if E[vj][f][ui][e] == !0 {
                                E[vj][f][ui][e] = cnf.var();
                            }
                            E[ui][e][vj][f] = E[vj][f][ui][e];
                        }
                    }
                }
            }
        }
    }
    for ui in 0..n * D {
        for e in 0..6 {
            let mut tmp = vec![];
            for vj in 0..n * D {
                for f in 0..6 {
                    if E[ui][e][vj][f] != !0 {
                        tmp.push(E[ui][e][vj][f]);
                    }
                }
            }
            cnf.choose_one(&tmp);
        }
    }
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                for f in 0..6 {
                    if E[u * D][e][v * D][f] == !0 {
                        continue;
                    }
                    for i1 in 0..D {
                        for i2 in 0..D {
                            if i1 == i2 {
                                continue;
                            }
                            for j1 in 0..D {
                                for j2 in 0..D {
                                    for f2 in 0..6 {
                                        if f == f2 {
                                            continue;
                                        }
                                        // E[u * D + i1][e][v * D + j1][f] -> !E[u * D + i2][e][v * D + j2][f2]
                                        cnf.clause([
                                            -E[u * D + i1][e][v * D + j1][f],
                                            -E[u * D + i2][e][v * D + j2][f2],
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
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
                    // V[t][i] & E[u * D + i][e][v * D + j][f] -> V[t+1][j]
                    for f in 0..6 {
                        cnf.clause([-V[t][i], -E[u * D + i][e][v * D + j][f], V[t + 1][j]]);
                    }
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
    // assert_eq!(cnf.sat.solve(), Some(true));
    solve_no_marks::solve_cnf_parallel(&mut cnf, 25, 25);
    let mut guess = Guess {
        start: super_guess.start * D,
        graph: vec![[(!0, !0); 6]; judge.num_rooms()],
        rooms: vec![0; judge.num_rooms()],
    };
    for u in 0..n {
        for i in 0..D {
            guess.rooms[u * D + i] = super_guess.rooms[u];
        }
    }
    for u in 0..n * D {
        for e in 0..6 {
            for v in 0..n * D {
                for f in 0..6 {
                    if E[u][e][v][f] != !0 && cnf.sat.value(E[u][e][v][f]) == Some(true) {
                        assert!(guess.graph[u][e] == (!0, !0));
                        assert!(cnf.sat.value(E[v][f][u][e]) == Some(true));
                        guess.graph[u][e] = (v, f);
                    }
                }
            }
        }
    }
    assert!(judge.guess(&guess));
}
