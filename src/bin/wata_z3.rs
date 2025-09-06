use icfpc2025::judge::*;
use rand::prelude::*;

fn main() {
    let mut judge = get_judge_from_stdin();
    let mut rnd = rand::rng();

    let n = judge.num_rooms();

    //"0"~"5"の長さqのランダムな文字列Sを生成
    let mut route = vec![];
    for _ in 0..(n * 18) {
        let c: usize = rnd.random_range(0..6);
        route.push(c);
    }
    let label = judge.explore(&vec![route.clone()])[0].clone();
    solve(n, &label, &route);
    /*
    n: usize
    label: Vec<usize>
    route: Vec<usize>

    V[i] := i番目に訪れた頂点 [0, n)
    L[u] := 頂点uのラベル [0, 4)
    E[6u+e] := 頂点uのドアeの行き先 [0,6n)
    E[E[x]] = x
    L[V[i]]=label[i]
    6V[i+1]<=E[6V[i]+route[i]]<6V[i+1]
     */
}
use z3::ast::{Array, Ast, Int};
use z3::{Config, Context, SatResult, Solver, Sort};

/// 問題を解き、結果を出力する関数
fn solve(n: usize, label: &[usize], route: &[usize]) {
    let solver = Solver::new();
    let num_steps = label.len();
    // 2. Z3の変数を定義
    // V[i] := i番目に訪れた頂点 [0, n)
    let v: Vec<Int> = (0..num_steps)
        .map(|i| Int::new_const(format!("v_{}", i)))
        .collect();

    // Z3のIntソート（型）を定義
    let int_sort = Sort::int();

    // L[u] := 頂点uのラベル [0, 4)
    // Z3のArray型 (Int -> Int)としてモデル化
    let l = Array::new_const("L", &int_sort, &int_sort);

    // E[6u+e] := 頂点uのドアeの行き先 [0, 6n)
    // Z3のArray型 (Int -> Int)としてモデル化
    let e_arr = Array::new_const("E", &int_sort, &int_sort);

    // 3. 制約をソルバーに追加

    // 制約: 0 <= V[i] < n
    for v_i in &v {
        solver.assert(&v_i.ge(&Int::from_u64(0)));
        solver.assert(&v_i.lt(&Int::from_u64(n as u64)));
    }

    // 制約: 0 <= L[u] < 4 (for u in 0..n)
    for i in 0..n {
        let u = Int::from_u64(i as u64);
        let l_u = l.select(&u).as_int().unwrap();
        solver.assert(&l_u.ge(&Int::from_u64(0)));
        solver.assert(&l_u.lt(&Int::from_u64(4)));
    }

    // 制約: E[E[x]] = x および 0 <= E[x] < 6n (for x in 0..6n)
    for i in 0..(6 * n) {
        let x = Int::from_u64(i as u64);
        let e_x = e_arr.select(&x).as_int().unwrap();

        // 0 <= E[x] < 6n
        solver.assert(&e_x.ge(&Int::from_u64(0)));
        solver.assert(&e_x.lt(&Int::from_u64((6 * n) as u64)));

        // E[E[x]] = x
        let e_e_x = e_arr.select(&e_x);
        solver.assert(&e_e_x.eq(&x));
    }

    // 制約: L[V[i]] = label[i]
    for i in 0..num_steps {
        let v_i = &v[i];
        let label_i = Int::from_u64(label[i] as u64);
        solver.assert(&l.select(v_i).eq(&label_i));
    }

    // 制約: 6*V[i+1] <= E[6*V[i] + route[i]] < 6*(V[i+1] + 1)
    for i in 0..route.len() {
        let route_i = Int::from_u64(route[i] as u64);
        let from_door = 6 * &v[i] + route_i;
        let to_door = e_arr.select(&from_door).as_int().unwrap();
        let lower_bound = 6 * &v[i + 1];
        let upper_bound = 6 * (&v[i + 1] + 1);
        solver.assert(&to_door.ge(&lower_bound));
        solver.assert(&to_door.lt(&upper_bound));
    }

    // 4. 解を求める
    println!("Solving...");
    match solver.check() {
        SatResult::Sat => {
            println!("\nSAT: Found a solution!");
            let model = solver.get_model().unwrap();
            print_solution(&model, n, &v, &l, &e_arr);
        }
        SatResult::Unsat => println!("\nUnsat: No solution found for the given constraints."),
        SatResult::Unknown => println!("\nUnknown: The solver could not determine satisfiability."),
    }
}

/// 見つかった解を整形して表示する関数
fn print_solution(model: &z3::Model, n: usize, v: &[Int], l: &Array, e_arr: &Array) {
    // V の値を表示
    println!("--------------------");
    println!("V (Visited Vertices Sequence):");
    let v_values: Vec<i64> = v
        .iter()
        .map(|v_i| model.eval(v_i, true).unwrap().as_i64().unwrap())
        .collect();
    println!("{:?}", v_values);

    // L の値を表示
    println!("\nL (Vertex Labels):");
    let mut l_values = vec![0; n];
    print!("[");
    for i in 0..n {
        let u = Int::from_u64(i as u64);
        let val = model
            .eval(&l.select(&u).as_int().unwrap(), true)
            .unwrap()
            .as_i64()
            .unwrap();
        l_values[i] = val;
        print!("v{}: {}, ", i, val);
    }
    println!("]");

    // E の値を表示
    println!("\nE (Graph Edges):");
    let mut e_values = vec![0; 6 * n];
    for i in 0..(6 * n) {
        let x = Int::from_u64(i as u64);
        e_values[i] = model
            .eval(&e_arr.select(&x).as_int().unwrap(), true)
            .unwrap()
            .as_i64()
            .unwrap();
    }

    for u in 0..n {
        println!("  Vertex {}:", u);
        for d in 0..6 {
            let from_door_idx = u * 6 + d;
            let to_door_val = e_values[from_door_idx] as usize;
            let to_vertex = to_door_val / 6;
            let to_door_idx = to_door_val % 6;
            println!(
                "    - Door {} connects to Door {} of Vertex {}",
                d, to_door_idx, to_vertex
            );
        }
    }
    println!("--------------------");
}
