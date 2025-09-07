#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]

use icfpc2025::{
    judge::Guess,
    solve_no_marks::{self, Cnf},
    *,
};
use itertools::Itertools;
use rand::prelude::*;
use tokio::time::error::Elapsed;

fn balanced_plan(len: usize, m: usize, rng: &mut impl Rng) -> Vec<usize> {
    let mut plan = Vec::with_capacity(len);
    for d in 0..len {
        plan.push(d % m);
    }
    plan.shuffle(rng);
    plan
}

fn main() {
    let mut rng = rand::rng();
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let D = 2; // 倍化率
    let K = 3; // 全体のクエリ数
    let F = judge.num_rooms() * 2; // 前半パートの長さ
    let n = judge.num_rooms() / D;
    let (plans, labels) = {
        let mut plans = vec![];
        let mut first = 0;
        let mut plans0 = vec![];
        for k in 0..K {
            let tmp = balanced_plan(judge.num_rooms() * 6, 6, &mut rng);
            plans.push(tmp.iter().map(|&d| (None, d)).collect_vec());
            if first + judge.num_rooms() * 6 <= F {
                first += judge.num_rooms() * 6;
                plans0.push(tmp);
            } else {
                let f = F - first;
                first += f;
                let mut b = balanced_plan(judge.num_rooms() * 6 - f, 4, &mut rng);
                for p in f..judge.num_rooms() * 6 {
                    plans[k][p].0 = b.pop();
                }
                if f > 0 {
                    plans0.push(tmp[..f].to_vec());
                }
            }
        }
        let mut labels = judge.explore(&plans);
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
    let mut cnf = Cnf::new();

    // V[t][i] := 時刻 t に訪れたのは (u,i) である
    let mut V = mat![!0; labels.len(); D];
    for t in 0..labels.len() {
        for d in 0..D {
            V[t][d] = cnf.var();
        }
        // 時刻 t にはどれか一つの (u,i) にいる
        cnf.choose_one(&V[t]);
    }

    // E[u][e][v][f] := 頂点uのe番目のドアが頂点vのf番目のドアに繋がっている
    let mut E = mat![0; n; 6; n; 6];

    // E[u][e][v][f] := u の e 番目のドアが v の f 番目 を結ぶ
    for ui in 0..n * D {
        for e in 0..6 {
            let mut tmp = vec![];
            for vj in 0..n * D {
                for f in 0..6 {
                    if E[vj][f][ui][e] == !0 {
                        E[vj][f][ui][e] = cnf.var();
                    }
                    // 反対の辺は同じもの
                    E[ui][e][vj][f] = E[vj][f][ui][e];
                    // D = 2 決め打ちだと、E[u][i][v][j] が有効な時、 E[v^1][i][u^1][j] も有効
                    // つまり同じものだと見做せる
                    E[vj ^ 1][e][ui ^ 1][f] = E[vj][f][ui][e];
                    // 逆向きも同じもの
                    E[ui ^ 1][f][vj ^ 1][e] = E[vj][f][ui][e];
                }
            }

            // u の e 番目のドアはどれか一つの (v,j) と結ぶ
            cnf.choose_one(&tmp);
        }
    }

    //各時間について解く
    //色は001122330011....のように2つずつ並ぶ
    //なので最初の部屋は色labels[0]なので、最初の部屋はlabels[0]*2と決め打って良い
    let first_room = labels[0] * 2;

    // いる場所Vについての制約
    // V[t][u*D+i] := 時刻 t の開始時点で、存在するのは (u,i) である
    // 最初の部屋は first_room にいる。uは現在の部屋
    let mut u = first_room;
    cnf.clause([V[0][u]]);
    for t in 0..plans.len() {
        //plants[t].1 == !0 のときは区切りなので最初に戻る
        if plans[t].1 == !0 {
            u = first_room;
            cnf.clause([V[t + 1][0]]);
        } else {
            // 時刻tではドアeを選択する
            let (_, e) = plans[t];

            for ui in 0..6 * D {
                for tj in 0..6 * D {
                    // 時刻 t に (u,i) にいて、ドア e を選ぶと、時刻 t+1 には (t,j) にいる
                    for f in 0..6 {
                        // V[t][ui] -> E[ui][e][tj][f] -> V[t+1][tj]
                        cnf.clause([-V[t][ui], -E[ui][e][tj][f], V[t + 1][tj]]);
                    }
                }
            }
            u = v;
        }
    }

    // 色についての制約
    u = first_room;
    // prev_t[u] := u に最後に訪れた時刻
    // C[t][ui][c] := 時刻 t の開始時点で、(u,i) の色が c である
    let mut C = mat![!0; plans.len() + 1; n * D; 4];
    // 最初の部屋の色は最初に決まっている
    for ui in 0..n * D {
        for c in 0..4 {
            C[0][ui][c] = cnf.var();
        }
        cnf.clause([C[0][ui][ui / 2 % 4]]);
    }

    // 各ターンの色の更新
    for t in 0..plans.len() {
        for ui in 0..n * D {
            for c in 0..4 {
                C[t][ui][c] = cnf.var();
            }
            // uiの色は時間tについて一つに定まる
            cnf.choose_one(&C[t][ui]);
        }

        if let Some(new_c) = plans[t].0 {
            // 時間tに色を塗る場合
            for ui in 0..n * D {
                // V[t][ui] -> C[t][ui][new_c]
                // !V[t][ui] -> !C[t][ui][c]
                for c in 0..4 {
                    if c == new_c {
                        cnf.clause([-V[t][ui], C[t][ui][new_c]]);
                    } else {
                        cnf.clause([V[t][ui], -C[t][ui][c]]);
                    }
                }
            }
        } else {
            // 色を塗らない場合
            for ui in 0..n * D {
                for c in 0..4 {
                    // 単純に前ターンのCを引き継げばよい
                    // C[t-1][ui][c] -> C[t][ui][c]
                    cnf.clause([-C[t - 1][ui][c], C[t][ui][c]]);
                    // !C[t-1][ui][c] -> !C[t][ui][c]
                    cnf.clause([C[t - 1][ui][c], -C[t][ui][c]]);
                }
            }
        }
    }

    //　各ターンの色の整合性
    for t in 0..labels.len() {
        for ui in 0..n * D {
            for c in 0..4 {
                // 時刻 t に (u,i) にいるなら、その色は C[t][ui][c] である
                // V[t][ui] -> C[t][ui][c]
                cnf.clause([-V[t][ui], C[t][ui][c]]);
            }
        }
    }

    assert_eq!(cnf.sat.solve(), Some(true));
    let mut guess = Guess {
        start: super_guess.start * D,
        graph: vec![[(!0, !0); 6]; judge.num_rooms()],
        rooms: vec![0; judge.num_rooms()],
    };

    //初期の色は0011223300....のようにDつずつ並ぶ
    for ui in 0..n * D {
        guess.rooms[ui] = ui / D % 4;
    }

    // グラフの復元
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
