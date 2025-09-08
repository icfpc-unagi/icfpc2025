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

fn gacha(n: usize, plan: &[(Option<usize>, usize)], labels: &[usize]) -> f64 {
    let mut label_door = mat![0; 4; 6];
    for i in 0..labels.len() - 1 {
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

fn gacha2(n: usize, plan: &[(Option<usize>, usize)], labels: &[usize]) -> f64 {
    let mut muda = 0.0;
    let mut now = 1.0;
    for i in 0..plan.len() {
        if plan[i].1 == !0 {
            continue;
        }
        if let Some(c) = plan[i].0 {
            if labels[i] == c {
                muda += now;
            } else {
                muda += 0.0;
            }
            now = now * (n as f64 - 1.0) / n as f64;
        }
    }
    dbg!(muda);
    muda
}

fn main() {
    let mut rng = rand::rng();
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    let D = 2; // 倍化率
    let K = 1; // 全体のクエリ数
    let F = judge.num_rooms() * 3 / 2; // 前半パートの長さ
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

        for k in 0..K {
            let score = gacha(n, &plans[k], &labels[k]);
            let score2 = gacha2(n, &plans[k], &labels[k]);

            eprintln!("gacha score {}: {} {}", k, score, score2);
            if score >= 0.0025 || score2 >= 1.5 {
                //panic!("unlucky");
                eprintln!("unlucky");
            }
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
    let mut V = mat![!0; labels.len(); n * D];
    for t in 0..labels.len() {
        for d in 0..n * D {
            V[t][d] = cnf.var();
        }
        // 時刻 t にはどれか一つの (u,i) にいる
        cnf.choose_one(&V[t]);
    }

    // 最初の部屋は labels[0] * D で決まっている
    let first_room = labels[0] * D;
    cnf.clause([V[0][first_room]]);

    // A[u][e][v] := u の ドア e は v とつながる
    let mut A = mat![!0; n * D; 6; n * D];
    for u in 0..n * D {
        for e in 0..6 {
            for v in 0..n * D {
                A[u][e][v] = cnf.var();
            }
            cnf.choose_one(&A[u][e]);
        }
    }

    // E[u][e][v][f] := 頂点uのe番目のドアが頂点vのf番目のドアに繋がっている
    let mut E = mat![!0; n * D; 6; n * D; 6];

    // E[u][e][v][f] := u の e 番目のドアが v の f 番目 を結ぶ
    for ui in 0..n * D {
        for e in 0..6 {
            let mut tmp = vec![];
            for vj in 0..n * D {
                for f in 0..6 {
                    // 出入口が違う自己ループは出入り口が一緒な自己ループ2つに置き換えられるので禁止
                    if ui == vj && e != f {
                        continue;
                    }

                    if E[vj][f][ui][e] == !0 {
                        E[vj][f][ui][e] = cnf.var();
                        // 反対の辺は同じもの
                        E[ui][e][vj][f] = E[vj][f][ui][e];
                        // D = 2 決め打ちだと、E[u][i][v][j] が有効な時、 E[v^1][i][u^1][j] も有効
                        // つまり同じものだと見做せる
                        E[vj ^ 1][f][ui ^ 1][e] = E[vj][f][ui][e];
                        // 逆向きも同じもの
                        E[ui ^ 1][e][vj ^ 1][f] = E[vj][f][ui][e];
                    }
                    tmp.push(E[ui][e][vj][f]);

                    //AとEの整合性
                    //EがこれならAがこれみたいな
                    // E[u][e][v][f] -> A[u][e][v]
                    cnf.clause([-E[ui][e][vj][f], A[ui][e][vj]]);
                }
            }

            // u の e 番目のドアはどれか一つの (v,j) と結ぶ
            cnf.choose_one(&tmp);
        }
    }

    // v の f 番目のドアに結ぶ u の e 番目のドアはどれか一つ
    for v in 0..n * D {
        for f in 0..6 {
            let mut col = vec![];
            for u in 0..n * D {
                for e in 0..6 {
                    col.push(E[u][e][v][f]);
                }
            }
            cnf.choose_one(&col);
        }
    }

    //各時間について解く
    //色は001122330011....のように2つずつ並ぶ
    //なので最初の部屋は色labels[0]なので、最初の部屋はlabels[0]*2と決め打って良い

    // いる場所Vについての制約

    for t in 0..plans.len() {
        //plants[t].1 == !0 のときは区切りなので最初に戻る
        if plans[t].1 == !0 {
            cnf.clause([V[t + 1][first_room]]);
        } else {
            // 時刻tではドアeを選択する
            let (_, e) = plans[t];

            for ui in 0..n * D {
                for tj in 0..n * D {
                    // 時刻 t に (u,i) にいて、ドア e を選ぶと、時刻 t+1 には (t,j) にいる
                    for f in 0..6 {
                        // V[t][ui] -> E[ui][e][tj][f] -> V[t+1][tj]
                        //cnf.clause([-V[t][ui], -E[ui][e][tj][f], V[t + 1][tj]]);

                        //EではなくAを使う
                        // V[t][ui] & A[ui][e][tj] -> V[t+1][tj]
                        cnf.clause([-V[t][ui], -A[ui][e][tj], V[t + 1][tj]]);
                    }
                }
            }
        }
    }

    // 色についての制約
    let mut C = mat![!0; plans.len() + 1; n * D; 4];
    // 最初の部屋の色は最初に決まっている
    for ui in 0..n * D {
        for c in 0..4 {
            C[0][ui][c] = cnf.var();
            if c == ui / D % 4 {
                // 最初の部屋の色は ui/D%4 で決まっている
                cnf.clause([C[0][ui][c]]);
            } else {
                // 最初の部屋の色は ui/D%4 で決まっている
                cnf.clause([-C[0][ui][c]]);
            }
        }
    }

    // 各ターンの色の更新
    for t in 0..plans.len() {
        for ui in 0..n * D {
            for c in 0..4 {
                C[t + 1][ui][c] = cnf.var();
            }
            // uiの色は時間tについて一つに定まる
            cnf.choose_one(&C[t + 1][ui]);
        }

        if let Some(new_c) = plans[t].0 {
            for ui in 0..n * D {
                // V[t][ui] => C[t+1][ui][new_c]
                cnf.clause([-V[t][ui], C[t + 1][ui][new_c]]);
                // V[t][ui] => !C[t+1][ui][c]  (c != new_c)
                for c in 0..4 {
                    if c != new_c {
                        cnf.clause([-V[t][ui], -C[t + 1][ui][c]]);
                    }

                    // 正色の持ち上げ
                    cnf.clause([V[t][ui], -C[t][ui][c], C[t + 1][ui][c]]);
                    // 反色の持ち上げ
                    cnf.clause([V[t][ui], C[t][ui][c], -C[t + 1][ui][c]]);
                }
            }
        } else {
            // 色を塗らない場合
            for ui in 0..n * D {
                for c in 0..4 {
                    // 単純に前ターンのCを引き継げばよい
                    // C[t][ui][c] -> C[t+1][ui][c]
                    cnf.clause([-C[t][ui][c], C[t + 1][ui][c]]);
                    // !C[t][ui][c] -> !C[t+1][ui][c]
                    cnf.clause([C[t][ui][c], -C[t + 1][ui][c]]);
                }
            }
        }
    }

    //　各ターンの色の整合性
    for t in 0..labels.len() {
        for ui in 0..n * D {
            for c in 0..4 {
                if c != labels[t] {
                    // V[t][ui] -> !C[t][ui][c]
                    cnf.clause([-V[t][ui], -C[t][ui][c]]);
                } else {
                    // V[t][ui] -> C[t][ui][c]
                    cnf.clause([-V[t][ui], C[t][ui][c]]);
                }
            }
        }
    }

    // 解けたらうれしいな
    //assert_eq!(cnf.sat.solve(), Some(true));
    solve_no_marks::solve_cnf_parallel(&mut cnf, 25, 25);

    let mut guess = Guess {
        start: first_room,
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

    // labels[i]と一致した答えが出ているか、実際にシミュレーションしてみる
    let mut now_room = first_room;
    let mut now_room_color = guess.rooms.clone();

    eprintln!("色チェックをするよ");
    for t in 0..plans.len() {
        let now_color = now_room_color[now_room];
        if now_color != labels[t] {
            eprintln!(
                "色が合わないよ: t = {}, now_room = {}, now_color = {}, labels[t] = {}",
                t, now_room, now_color, labels[t]
            );
        }

        let (new_c, e) = plans[t];
        if new_c.is_some() {
            // 色が決まっている場合
            let new_c = new_c.unwrap();
            now_room_color[now_room] = new_c;
        }
        // ドアを通る
        now_room = guess.graph[now_room][e].0;
    }

    assert!(judge.guess(&guess));
}
