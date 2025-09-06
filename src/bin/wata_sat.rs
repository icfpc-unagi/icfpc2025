#![allow(
    clippy::needless_range_loop,
    clippy::useless_vec,
    clippy::partialeq_to_none,
    clippy::ptr_arg,
    clippy::if_same_then_else,
    clippy::cloned_ref_to_slice_refs,
    non_snake_case,
    unused_variables
)]
use icfpc2025::{judge::*, *};

struct Counter {
    cnt: i32,
}

impl Counter {
    fn new() -> Self {
        Self { cnt: 0 }
    }
    fn next(&mut self) -> i32 {
        self.cnt += 1;
        self.cnt
    }
}

// 小さいときのAMO（ペアワイズ）
fn amo_pairwise(sat: &mut cadical::Solver, xs: &[i32]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            sat.add_clause([-xs[i], -xs[j]]);
        }
    }
}

// 大きいときのAMO（逐次: Sinz 2005 / ladder）
// 変数: s[0..k-2] （k=len(xs)）
// 節:
//  (¬x1 ∨ s1)
//  ∀i=2..k-1: (¬xi ∨ si)
//  ∀i=2..k:   (¬xi ∨ ¬s_{i-1})
//  ∀i=2..k-1: (¬s_{i-1} ∨ s_i)
fn amo_sequential(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    let k = xs.len();
    if k <= 1 {
        return;
    } // 0 or 1 個なら AMO は自明

    // s[i] は 1-based の s_{i+1} に対応（i=0..k-2）
    let mut s = Vec::with_capacity(k - 1);
    for _ in 0..(k - 1) {
        s.push(id.next());
    }

    // (¬x1 ∨ s1)
    sat.add_clause([-xs[0], s[0]]);
    // ∀i=2..k-1: (¬xi ∨ si)  → i = 1..k-2
    for i in 1..k - 1 {
        sat.add_clause([-xs[i], s[i]]);
    }
    // ∀i=2..k: (¬xi ∨ ¬s_{i-1}) → i = 1..k-1, 参照は s[i-1]
    for i in 1..k {
        sat.add_clause([-xs[i], -s[i - 1]]);
    }
    // ∀i=2..k-1: (¬s_{i-1} ∨ s_i) → i = 1..k-2, 参照は s[i-1], s[i]
    for i in 1..k - 1 {
        sat.add_clause([-s[i - 1], s[i]]);
    }
}

/// ちょうど1: ALO + AMO（小規模はペアワイズ、大規模は逐次）
/// xs は空でないこと（空だと UNSAT）。
fn choose_one(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    // ALO（少なくとも1）
    sat.add_clause(xs.iter().copied());

    // AMO（高々1）: 閾値は適宜調整。だいたい 6〜8 あたりが無難。
    if xs.len() <= 6 {
        amo_pairwise(sat, xs);
    } else {
        amo_sequential(sat, xs, id);
    }
}

#[allow(unused)]
fn first_use_SBP(sat: &mut cadical::Solver, V: &Vec<Vec<i32>>, id: &mut Counter) {
    let n = V.len();
    let m = V[0].len();
    // 補助変数: z[i][u] = 「i が集合 u の first-use」
    //           p[i][u] = 「i までに集合 u は登場したか（z[0..=i][u] のOR）」
    let mut z = vec![vec![0i32; m]; n];
    let mut p = vec![vec![0i32; m]; n];
    for u in 0..m {
        for i in 0..n {
            z[i][u] = id.next();
            p[i][u] = id.next();
        }
    }

    // 定義と連結：
    // 1) V[i][u] -> p[i][u]
    // 2) z[i][u] -> V[i][u]
    // 3) z[i][u] -> p[i][u]
    // 4) i==0: p[0][u] <-> z[0][u]
    //    i>0 : (a) p[i-1][u] -> p[i][u]        （単調増加）
    //          (b) p[i][u] -> p[i-1][u] ∨ z[i][u]  （緊密な定義）
    //          (c) z[i][u] -> ¬p[i-1][u]      （「最初」性）
    for u in 0..m {
        for i in 0..n {
            // V[i][u] -> p[i][u]
            sat.add_clause([-V[i][u], p[i][u]]);
            // z[i][u] -> V[i][u]
            sat.add_clause([-z[i][u], V[i][u]]);
            // z[i][u] -> p[i][u]
            sat.add_clause([-z[i][u], p[i][u]]);

            if i == 0 {
                // p[0][u] <-> z[0][u]
                sat.add_clause([-p[0][u], z[0][u]]);
                sat.add_clause([-z[0][u], p[0][u]]);
            } else {
                // 単調: p[i-1][u] -> p[i][u]
                sat.add_clause([-p[i - 1][u], p[i][u]]);
                // 緊密: p[i][u] -> p[i-1][u] ∨ z[i][u]
                sat.add_clause([-p[i][u], p[i - 1][u], z[i][u]]);
                // first-use: z[i][u] -> ¬p[i-1][u]
                sat.add_clause([-z[i][u], -p[i - 1][u]]);
            }
        }
    }

    // 集合の登場順を強制: すべての i, u>=1 で p[i][u] -> p[i][u-1]
    // （集合uが i までに登場しているなら、u-1 も i までに登場している）
    for u in 1..m {
        for i in 0..n {
            sat.add_clause([-p[i][u], p[i][u - 1]]);
        }
    }
}

fn main() {
    let judge = get_judge_from_stdin_with(true);
    let fix_label = true;
    let use_diff = true;
    let use_same = false;

    let n = judge.num_rooms();

    // Use pre-recorded explores instead of generating random route
    let explored = judge.explored();
    assert!(
        !explored.plans.is_empty(),
        "explored is empty; provide explores via JSON"
    );
    let plan: Vec<usize> = explored.plans[0].iter().map(|&(_, d)| d).collect();
    let labels = explored.results[0].clone();

    let mut diff = mat![false; labels.len(); labels.len()];
    loop {
        let bk = diff.clone();
        for i in 0..labels.len() {
            for j in i + 1..labels.len() {
                if labels[i] != labels[j] {
                    diff[i][j] = true;
                    diff[j][i] = true;
                } else if j < plan.len() && plan[i] == plan[j] && diff[i + 1][j + 1] {
                    diff[i][j] = true;
                    diff[j][i] = true;
                }
            }
        }
        if bk == diff {
            break;
        }
    }

    let mut sat: cadical::Solver = cadical::Solver::with_config("sat").unwrap();
    let mut id = Counter::new();

    // V[i][u] := i番目に訪れたのが頂点uである
    let mut V = mat![0; labels.len(); n];
    for i in 0..labels.len() {
        for u in 0..n {
            V[i][u] = id.next();
        }
        choose_one(&mut sat, &V[i], &mut id);
    }

    if use_diff {
        for i in 0..labels.len() {
            for j in 0..labels.len() {
                if diff[i][j] {
                    for u in 0..n {
                        sat.add_clause([-V[i][u], -V[j][u]]);
                    }
                }
            }
        }
    }

    // first_use_SBP(&mut sat, &V, &mut id);

    // L[u][k] := 頂点uのラベルがkである
    let mut L = mat![0; n; 4];
    for u in 0..n {
        for k in 0..4 {
            L[u][k] = id.next();
        }
        choose_one(&mut sat, &L[u], &mut id);
    }

    if fix_label {
        let mut first = vec![false; 4];
        for i in 0..labels.len() {
            if first[labels[i]].setmax(true) {
                sat.add_clause([V[i][labels[i]]]);
            }
        }
        for u in 0..n {
            sat.add_clause([L[u][u % 4]]);
        }
    }

    // E[u][e][v][f] := 頂点uのe番目のドアが頂点vのf番目のドアに繋がっている
    let mut E = mat![0; n; 6; n; 6];
    for u in 0..n {
        for e in 0..6 {
            let mut tmp = vec![];
            for v in 0..n {
                for f in 0..6 {
                    if (u, e) <= (v, f) {
                        E[u][e][v][f] = id.next();
                    } else {
                        E[u][e][v][f] = E[v][f][u][e];
                    }
                    tmp.push(E[u][e][v][f]);
                }
            }
            choose_one(&mut sat, &tmp, &mut id);
        }
    }

    // ラベルが一致
    for i in 0..labels.len() {
        for u in 0..n {
            let k = labels[i];
            sat.add_clause([-V[i][u], L[u][k]]);
        }
    }

    // 遷移に対応する辺が存在
    for i in 0..plan.len() {
        let e = plan[i];
        for u in 0..n {
            for v in 0..n {
                sat.add_clause([
                    -V[i][u],
                    -V[i + 1][v],
                    E[u][e][v][0],
                    E[u][e][v][1],
                    E[u][e][v][2],
                    E[u][e][v][3],
                    E[u][e][v][4],
                    E[u][e][v][5],
                ]);
            }
        }
    }

    if use_same {
        let mut S = mat![0; labels.len(); labels.len()];
        for i in 0..labels.len() {
            for j in i..labels.len() {
                S[i][j] = id.next();
                S[j][i] = S[i][j];
                if diff[i][j] {
                    sat.add_clause([-S[i][j]]);
                }
            }
            sat.add_clause([S[i][i]]);
        }
        for i in 0..plan.len() {
            for j in i + 1..plan.len() {
                if diff[i][j] {
                    continue;
                }
                if plan[i] == plan[j] {
                    // S[i][j] -> S[i+1][j+1]
                    sat.add_clause([-S[i][j], S[i + 1][j + 1]]);
                }
                for u in 0..n {
                    // S[i][j] -> (V[i][u] <-> V[j][u])
                    sat.add_clause([-S[i][j], -V[i][u], V[j][u]]);
                    sat.add_clause([-S[i][j], V[i][u], -V[j][u]]);
                    sat.add_clause([S[i][j], -V[i][u], -V[j][u]]);
                }
            }
        }
    }

    eprintln!("num_vars = {}", sat.num_variables());
    eprintln!("num_clauses = {}", sat.num_clauses());

    assert_eq!(sat.solve(), Some(true));

    let mut guess = Guess {
        start: 0,
        rooms: vec![0; n],
        graph: vec![[(!0, !0); 6]; n],
    };
    guess.start = (0..n).find(|&u| sat.value(V[0][u]) == Some(true)).unwrap();
    for u in 0..n {
        for k in 0..4 {
            if sat.value(L[u][k]) == Some(true) {
                guess.rooms[u] = k;
            }
        }
        for e in 0..6 {
            guess.graph[u][e] = (u, e);
            for v in 0..n {
                for f in 0..6 {
                    if sat.value(E[u][e][v][f]) == Some(true) {
                        guess.graph[u][e] = (v, f);
                    }
                }
            }
        }
    }
    assert!(check_explore(&guess, &[plan.clone()], &[labels.clone()]));
    judge.guess(&guess);
    let mut es = vec![];
    for u in 0..n {
        for e in 0..6 {
            if u < guess.graph[u][e].0 {
                es.push((u, guess.graph[u][e].0));
            }
        }
    }
    eprintln!("{} {}", n, es.len());
    for (u, v) in es {
        eprintln!("{} {}", u, v);
    }
    dbg!(&guess.rooms);
}
