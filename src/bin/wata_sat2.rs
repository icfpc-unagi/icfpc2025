// Cargo.toml
// [dependencies]
// rustsat = "0.7"
// rustsat-minisat = "0.7"   // もしくは rustsat-cadical / rustsat-kissat 等

#![allow(
    clippy::needless_range_loop,
    clippy::useless_vec,
    clippy::partialeq_to_none,
    clippy::ptr_arg,
    clippy::if_same_then_else,
    clippy::cloned_ref_to_slice_refs,
    clippy::match_like_matches_macro,
    clippy::bool_comparison,
    non_snake_case,
    unused_variables
)]
use icfpc2025::{judge::*, *};
use rand::prelude::*;

use rustsat::clause;
use rustsat::instances::SatInstance;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Assignment, TernaryVal};
use std::sync::mpsc; // マクロ（テスト補助）: 可読性のため使用

/// rustsat のリテラル型
type Lit = rustsat::types::Lit;

struct LitCounter {
    // SatInstance.new_lit() を経由するので i32 カウンタは不要
}
impl LitCounter {
    fn new_lit(inst: &mut SatInstance) -> Lit {
        inst.new_lit()
    }
}

// 小さいときのAMO（ペアワイズ）
fn amo_pairwise(inst: &mut SatInstance, xs: &[Lit]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            inst.add_clause(clause![!xs[i], !xs[j]]);
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
fn amo_sequential(inst: &mut SatInstance, xs: &[Lit]) {
    let k = xs.len();
    if k <= 1 {
        return;
    }

    // s[i] は 1-based の s_{i+1} に対応（i=0..k-2）
    let mut s: Vec<Lit> = Vec::with_capacity(k - 1);
    for _ in 0..(k - 1) {
        s.push(LitCounter::new_lit(inst));
    }

    // (¬x1 ∨ s1)
    inst.add_clause(clause![!xs[0], s[0]]);
    // ∀i=2..k-1: (¬xi ∨ si)  → i = 1..k-2
    for i in 1..k - 1 {
        inst.add_clause(clause![!xs[i], s[i]]);
    }
    // ∀i=2..k: (¬xi ∨ ¬s_{i-1}) → i = 1..k-1
    for i in 1..k {
        inst.add_clause(clause![!xs[i], !s[i - 1]]);
    }
    // ∀i=2..k-1: (¬s_{i-1} ∨ s_i) → i = 1..k-2
    for i in 1..k - 1 {
        inst.add_clause(clause![!s[i - 1], s[i]]);
    }
}

/// ちょうど1: ALO + AMO（小規模はペアワイズ、大規模は逐次）
/// xs は空でないこと（空だと UNSAT）。
fn choose_one(inst: &mut SatInstance, xs: &[Lit]) {
    // ALO（少なくとも1）
    // clause! マクロは可変長不可なので、Vec<Lit> -> 手動で流し込む
    // SatInstance には k-項節を追加する API があるので、1節ずつ追加
    // 便宜上、配列をコピーして投入
    {
        let mut c = Vec::with_capacity(xs.len());
        c.extend_from_slice(xs);
        inst.add_clause(c.as_slice().into());
    }

    // AMO（高々1）
    if xs.len() <= 6 {
        amo_pairwise(inst, xs);
    } else {
        amo_sequential(inst, xs);
    }
}

#[allow(unused)]
fn first_use_SBP(inst: &mut SatInstance, V: &Vec<Vec<Lit>>) {
    let n = V.len();
    let m = V[0].len();
    // 補助変数: z[i][u] = 「i が集合 u の first-use」
    //           p[i][u] = 「i までに集合 u は登場したか（z[0..=i][u] のOR）」
    let mut z = vec![vec![Lit::positive(0); m]; n];
    let mut p = vec![vec![Lit::positive(0); m]; n];
    for u in 0..m {
        for i in 0..n {
            z[i][u] = LitCounter::new_lit(inst);
            p[i][u] = LitCounter::new_lit(inst);
        }
    }

    for u in 0..m {
        for i in 0..n {
            // V[i][u] -> p[i][u]
            inst.add_clause(clause![!V[i][u], p[i][u]]);
            // z[i][u] -> V[i][u]
            inst.add_clause(clause![!z[i][u], V[i][u]]);
            // z[i][u] -> p[i][u]
            inst.add_clause(clause![!z[i][u], p[i][u]]);

            if i == 0 {
                // p[0][u] <-> z[0][u]
                inst.add_clause(clause![!p[0][u], z[0][u]]);
                inst.add_clause(clause![!z[0][u], p[0][u]]);
            } else {
                // 単調: p[i-1][u] -> p[i][u]
                inst.add_clause(clause![!p[i - 1][u], p[i][u]]);
                // 緊密: p[i][u] -> p[i-1][u] ∨ z[i][u]
                {
                    let c = vec![!p[i][u], p[i - 1][u], z[i][u]];
                    inst.add_clause(c.as_slice().into());
                }
                // first-use: z[i][u] -> ¬p[i-1][u]
                inst.add_clause(clause![!z[i][u], !p[i - 1][u]]);
            }
        }
    }

    // 集合の登場順を強制: すべての i, u>=1 で p[i][u] -> p[i][u-1]
    for u in 1..m {
        for i in 0..n {
            inst.add_clause(clause![!p[i][u], p[i][u - 1]]);
        }
    }
}

fn lit_is_true(model: &Assignment, l: Lit) -> bool {
    let v = model.var_value(l.var());
    match (v, l.is_pos()) {
        (TernaryVal::True, true) => true,
        (TernaryVal::False, false) => true,
        _ => false,
    }
}

fn main() {
    let judge = get_judge_from_stdin_with(true);
    let fix_label = true;
    let use_diff = true;
    let use_same = false;

    let n = judge.num_rooms();

    let rng = rand_pcg::Pcg64Mcg::seed_from_u64(84300);
    let explores = judge.explored();
    let first = explores
        .first()
        .expect("explored is empty; provide explores via JSON");
    let plan = first.plans[0].clone();
    let labels = first.results[0].clone();

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

    // === ここから RustSAT ===
    let mut inst: SatInstance = SatInstance::new();

    // V[i][u] := i番目に訪れたのが頂点uである
    let mut V = mat![Lit::positive(0); labels.len(); n];
    for i in 0..labels.len() {
        for u in 0..n {
            V[i][u] = LitCounter::new_lit(&mut inst);
        }
        choose_one(&mut inst, &V[i]);
    }

    if use_diff {
        for i in 0..labels.len() {
            for j in 0..labels.len() {
                if diff[i][j] {
                    for u in 0..n {
                        inst.add_clause(clause![!V[i][u], !V[j][u]]);
                    }
                }
            }
        }
    }

    // first_use_SBP(&mut inst, &V);

    // L[u][k] := 頂点uのラベルがkである
    let mut L = mat![Lit::positive(0); n; 4];
    for u in 0..n {
        for k in 0..4 {
            L[u][k] = LitCounter::new_lit(&mut inst);
        }
        choose_one(&mut inst, &L[u]);
    }

    if fix_label {
        let mut first = vec![false; 4];
        for i in 0..labels.len() {
            if std::mem::replace(&mut first[labels[i]], true) == false {
                // sat.add_clause([V[i][labels[i]]]);
                inst.add_unit(V[i][labels[i]]);
            }
        }
        for u in 0..n {
            inst.add_unit(L[u][u % 4]);
        }
    }

    // E[u][e][v][f] := 頂点uのe番目のドアが頂点vのf番目のドアに繋がっている
    let mut E = mat![Lit::positive(0); n; 6; n; 6];
    for u in 0..n {
        for e in 0..6 {
            let mut tmp: Vec<Lit> = Vec::with_capacity(n * 6);
            for v in 0..n {
                for f in 0..6 {
                    if (u, e) <= (v, f) {
                        E[u][e][v][f] = LitCounter::new_lit(&mut inst);
                    } else {
                        E[u][e][v][f] = E[v][f][u][e];
                    }
                    tmp.push(E[u][e][v][f]);
                }
            }
            inst.add_clause(tmp.as_slice().into()); // ALO for one-of neighbors
            // AMO は大きいので逐次で
            amo_sequential(
                &mut inst,
                &E[u][e]
                    .iter()
                    .flat_map(|row| row.iter())
                    .copied()
                    .collect::<Vec<_>>(),
            );
        }
    }

    // ラベルが一致:  (¬V[i][u] ∨ L[u][labels[i]])
    for i in 0..labels.len() {
        for u in 0..n {
            inst.add_clause(clause![!V[i][u], L[u][labels[i]]]);
        }
    }

    // 遷移に対応する辺が存在
    for i in 0..plan.len() {
        let e = plan[i];
        for u in 0..n {
            for v in 0..n {
                let mut c = Vec::with_capacity(2 + 6);
                c.push(!V[i][u]);
                c.push(!V[i + 1][v]);
                for f in 0..6 {
                    c.push(E[u][e][v][f]);
                }
                inst.add_clause(c.as_slice().into());
            }
        }
    }

    if use_same {
        let mut S = mat![Lit::positive(0); labels.len(); labels.len()];
        for i in 0..labels.len() {
            for j in i..labels.len() {
                S[i][j] = LitCounter::new_lit(&mut inst);
                S[j][i] = S[i][j];
                if diff[i][j] {
                    inst.add_unit(!S[i][j]); // sat.add_clause([-S[i][j]])
                }
            }
            inst.add_unit(S[i][i]);
        }
        for i in 0..plan.len() {
            for j in i + 1..plan.len() {
                if diff[i][j] {
                    continue;
                }
                if plan[i] == plan[j] {
                    // S[i][j] -> S[i+1][j+1]
                    inst.add_clause(clause![!S[i][j], S[i + 1][j + 1]]);
                }
                for u in 0..n {
                    // S[i][j] -> (V[i][u] <-> V[j][u])
                    inst.add_clause(clause![!S[i][j], !V[i][u], V[j][u]]);
                    inst.add_clause(clause![!S[i][j], V[i][u], !V[j][u]]);
                    inst.add_clause(clause![S[i][j], !V[i][u], !V[j][u]]);
                }
            }
        }
    }

    // === 解く ===
    let (tx, rx) = mpsc::channel();
    {
        let tx = tx.clone();
        let inst = inst.clone();
        std::thread::spawn(move || {
            let mut solver = rustsat_minisat::core::Minisat::default();
            solver.add_cnf(inst.clone().into_cnf().0).unwrap();
            let res = solver.solve().unwrap();
            eprintln!("Minisat");
            assert!(matches!(res, SolverResult::Sat));
            let model = solver.full_solution().unwrap();
            tx.send(model).unwrap();
        });
    }
    {
        let tx = tx.clone();
        let inst = inst.clone();
        std::thread::spawn(move || {
            let mut solver = rustsat_cadical::CaDiCaL::default();
            solver.add_cnf(inst.clone().into_cnf().0).unwrap();
            let res = solver.solve().unwrap();
            eprintln!("CaDiCaL");
            assert!(matches!(res, SolverResult::Sat));
            let model = solver.full_solution().unwrap();
            tx.send(model).unwrap();
        });
    }
    {
        let tx = tx.clone();
        let inst = inst.clone();
        std::thread::spawn(move || {
            let mut solver = rustsat_glucose::core::Glucose::default();
            solver.add_cnf(inst.clone().into_cnf().0).unwrap();
            let res = solver.solve().unwrap();
            eprintln!("Glucose");
            assert!(matches!(res, SolverResult::Sat));
            let model = solver.full_solution().unwrap();
            tx.send(model).unwrap();
        });
    }
    {
        let tx = tx.clone();
        let inst = inst.clone();
        std::thread::spawn(move || {
            let mut solver = rustsat_kissat::Kissat::default();
            solver.add_cnf(inst.clone().into_cnf().0).unwrap();
            let res = solver.solve().unwrap();
            eprintln!("Kissat");
            assert!(matches!(res, SolverResult::Sat));
            let model = solver.full_solution().unwrap();
            tx.send(model).unwrap();
        });
    }
    let model = rx.recv().unwrap();

    // === モデルを読み取って Guess へ ===
    let mut guess = Guess {
        start: 0,
        rooms: vec![0; n],
        graph: vec![[(!0, !0); 6]; n],
    };

    // start
    guess.start = (0..n).find(|&u| lit_is_true(&model, V[0][u])).unwrap();

    // rooms
    for u in 0..n {
        for k in 0..4 {
            if lit_is_true(&model, L[u][k]) {
                guess.rooms[u] = k;
            }
        }
        for e in 0..6 {
            guess.graph[u][e] = (u, e);
            for v in 0..n {
                for f in 0..6 {
                    if lit_is_true(&model, E[u][e][v][f]) {
                        guess.graph[u][e] = (v, f);
                    }
                }
            }
        }
    }

    assert!(check_explore(&guess, &[plan.clone()], &[labels.clone()]));
    judge.guess(&guess);

    // デバッグ出力（元コード準拠）
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
    std::process::exit(0);
}
