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
    let D = 3; // 倍化率
    let K = 1; // 全体のクエリ数
    let FF = judge.num_rooms() * 2; // 前半パートの長さ
    let n = judge.num_rooms() / D;
    let (plans, labels) = {
        let mut plans = vec![];
        let mut first = 0;
        let mut plans0 = vec![];
        for k in 0..K {
            let tmp = balanced_plan(judge.num_rooms() * 6, 6, &mut rng);
            plans.push(tmp.iter().map(|&d| (None, d)).collect_vec());
            {
                let f = FF;
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
            {
                let f = FF;
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

    // E[u][e][v][f] := 頂点uのe番目のドアが頂点vのf番目のドアに繋がっている
    let mut E = mat![!0; n * D; 6; n * D; 6];
    let mut E2 = mat![!0; n * D; 6; n * D; 6];

    // E[u][e][v][f] := u の e 番目のドアが v の f 番目 を結ぶ
    // E2[u][e][v][f] := u の e 番目のドアが v の f 番目 を結ぶ
    // バリエーションがあるのでE/E2がどっちかあれば良い。どっちもはだめ。
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                for f in 0..6 {
                    if u > v || (u == v && e > f) {
                        continue;
                    }
                    //u/vについてD=3種類の候補を作る
                    let us = vec![u * 3, u * 3 + 1, u * 3 + 2];
                    let vs = vec![v * 3, v * 3 + 1, v * 3 + 2];
                    //6通りの組み合わせについて、E/E2の空いているほうに割り当てる
                    //6つのpermにおいて、D=2のとき各辺はちょうど2度現れる。(u=vかつe=fを除く)
                    for perm in (0..3).permutations(3) {
                        //全permutationを列挙
                        let u1 = us[perm[0]];
                        let u2 = us[perm[1]];
                        let u3 = us[perm[2]];
                        let v1 = vs[0];
                        let v2 = vs[1];
                        let v3 = vs[2];

                        //u == vの時は、入口＝出口になっているかチェックする
                        //問題で与えられる隠されたグラフは無向グラフ（入口＝出口になっている）であり、
                        //u==v, e==fの時は、u1-v2, u2-v3, u3-v1のような辺を貼ると入口と出口が対応しない
                        //これがダメなのは2種類のみ。自己ループは良いので(0,1,2)や(2,1,0)はOK
                        if u == v && e == f {
                            //u1-v2, u2-v3, u3-v1はだめ
                            if (u1 == v2) || (u2 == v3) || (u3 == v1) {
                                continue;
                            }
                            //u1-v3, u2-v1, u3-v2はだめ
                            if (u1 == v3) || (u2 == v1) || (u3 == v2) {
                                continue;
                            }
                        }

                        //u1<->v1を結ぶとき、u2<->v2, u3<->v3も結ぶため、3つの辺をまとめて一つの変数にする
                        let now_e = cnf.var();

                        if E[u1][e][v1][f] == !0 {
                            E[u1][e][v1][f] = now_e;
                        } else if E2[u1][e][v1][f] == !0 {
                            E2[u1][e][v1][f] = now_e;
                        } else {
                            eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", u1, e, v1, f);
                        }

                        if E[u2][e][v2][f] == !0 {
                            E[u2][e][v2][f] = now_e;
                        } else if E2[u2][e][v2][f] == !0 {
                            E2[u2][e][v2][f] = now_e;
                        } else {
                            eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", u2, e, v2, f);
                        }

                        if E[u3][e][v3][f] == !0 {
                            E[u3][e][v3][f] = now_e;
                        } else if E2[u3][e][v3][f] == !0 {
                            E2[u3][e][v3][f] = now_e;
                        } else {
                            eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", u3, e, v3, f);
                        }

                        // 逆向きの辺も張る 同一辺のときは張らない
                        if v1 != u1 || f != e {
                            if E[v1][f][u1][e] == !0 {
                                E[v1][f][u1][e] = now_e;
                            } else if E2[v1][f][u1][e] == !0 {
                                E2[v1][f][u1][e] = now_e;
                            } else {
                                eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", v1, f, u1, e);
                            }
                        }
                        if v2 != u2 || f != e {
                            if E[v2][f][u2][e] == !0 {
                                E[v2][f][u2][e] = now_e;
                            } else if E2[v2][f][u2][e] == !0 {
                                E2[v2][f][u2][e] = now_e;
                            } else {
                                eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", v2, f, u2, e);
                            }
                        }
                        if v3 != u3 || f != e {
                            if E[v3][f][u3][e] == !0 {
                                E[v3][f][u3][e] = now_e;
                            } else if E2[v3][f][u3][e] == !0 {
                                E2[v3][f][u3][e] = now_e;
                            } else {
                                eprintln!("バグ? 新しい辺を発見: {}-{} -> {}-{}", v3, f, u3, e);
                            }
                        }
                    }
                }
            }
        }
    }

    // v の f 番目のドアに結ぶ u の e 番目のドアはどれか一つ
    for v in 0..n * D {
        for f in 0..6 {
            let mut col = vec![];
            for u in 0..n * D {
                for e in 0..6 {
                    if E[u][e][v][f] != !0 {
                        col.push(E[u][e][v][f]);
                    }
                    if E2[u][e][v][f] != !0 {
                        col.push(E2[u][e][v][f]);
                    }
                }
            }
            col.sort();
            col.dedup();
            cnf.choose_one(&col);
        }
    }

    // u の e 番目のドアはどれか一つに結ぶ
    for u in 0..n * D {
        for e in 0..6 {
            let mut row = vec![];
            for v in 0..n * D {
                for f in 0..6 {
                    if E[u][e][v][f] != !0 {
                        row.push(E[u][e][v][f]);
                    }
                    if E2[u][e][v][f] != !0 {
                        row.push(E2[u][e][v][f]);
                    }
                }
            }
            row.sort();
            row.dedup();
            cnf.choose_one(&row);
        }
    }

    //各時間について解く
    //色は001122330011....のようにDつずつ並ぶ
    //なので最初の部屋は色labels[0]なので、最初の部屋はlabels[0]*Dと決め打って良い

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
                        if E[ui][e][tj][f] != !0 {
                            cnf.clause([-V[t][ui], -E[ui][e][tj][f], V[t + 1][tj]]);
                        }
                        if E2[ui][e][tj][f] != !0 {
                            cnf.clause([-V[t][ui], -E2[ui][e][tj][f], V[t + 1][tj]]);
                        }
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
                        if guess.graph[u][e] != (!0, !0) {
                            eprintln!(
                                "バグ? すでに辺が決まっている: {}-{} -> {:?}, but find {}-{} -> {} {}",
                                u, e, guess.graph[u][e], u, e, v, f,
                            );
                        }
                        if u == v && e == f {
                            eprintln!("自己ループを確認 at E {} {} -> {} {}", u, e, v, f);
                        }
                        assert!(guess.graph[u][e] == (!0, !0));
                        assert!(cnf.sat.value(E[v][f][u][e]) == Some(true));
                        guess.graph[u][e] = (v, f);
                    }
                    if E2[u][e][v][f] != !0 && cnf.sat.value(E2[u][e][v][f]) == Some(true) {
                        if guess.graph[u][e] != (!0, !0) {
                            eprintln!(
                                "バグ? すでに辺が決まっている: {}-{} -> {:?}, but find {}-{} -> {} {}",
                                u, e, guess.graph[u][e], u, e, v, f,
                            );
                        }
                        if u == v && e == f {
                            eprintln!("自己ループを確認 at E2 {} {} -> {} {}", u, e, v, f);
                        }
                        assert!(guess.graph[u][e] == (!0, !0));
                        assert!(cnf.sat.value(E2[v][f][u][e]) == Some(true));
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
        let (new_c, e) = plans[t];

        // ラベル整合性チェック
        let now_color = now_room_color[now_room];
        if now_color != labels[t] {
            eprintln!(
                "色が合わないよ: t = {}, now_room = {}, now_color = {}, labels[t] = {}",
                t, now_room, now_color, labels[t]
            );
        }

        // 色の塗り替え
        if let Some(c) = new_c {
            now_room_color[now_room] = c;
        }

        // セパレータは「初期位置にワープ」扱い。移動しない。
        if e == !0 {
            now_room = first_room;
            continue;
        }

        // 通常移動
        now_room = guess.graph[now_room][e].0;
    }

    assert!(judge.guess(&guess));
}
