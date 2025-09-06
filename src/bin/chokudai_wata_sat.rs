#![allow(
    clippy::needless_range_loop,
    clippy::useless_vec,
    clippy::partialeq_to_none,
    clippy::ptr_arg,
    clippy::needless_return,
    clippy::len_zero,
    clippy::unnecessary_mut_passed,
    clippy::cloned_ref_to_slice_refs,
    non_snake_case,
    unused_variables
)]
use std::collections::VecDeque;

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

struct Moves {
    label: Vec<usize>,
    door: Vec<usize>,
}

struct SameTable {
    table: Vec<Vec<usize>>, // table[i][j]: iとjが同じ部屋なら2, 違う部屋なら1, 不明なら0
    queue: VecDeque<(usize, usize)>,
}

impl SameTable {
    fn new(n: usize) -> Self {
        SameTable {
            table: vec![vec![0; n]; n],
            queue: VecDeque::new(),
        }
    }

    fn set_same(&mut self, i: usize, j: usize) {
        if i == j {
            return;
        }
        if self.table[i][j] == 1 {
            panic!("conflict at set_same: {}, {}", i, j);
        }
        if self.table[i][j] == 0 {
            //eprintln!("set_same: {}, {}", i, j);
            self.table[i][j] = 2;
            self.table[j][i] = 2;
            self.queue.push_back((i, j));
        }
    }

    fn set_not_same(&mut self, i: usize, j: usize) {
        if i == j {
            return;
        }
        if self.table[i][j] == 2 {
            panic!("conflict at set_not_same: {}, {}", i, j);
        }
        if self.table[i][j] == 0 {
            //eprintln!("set_not_same: {}, {}", i, j);
            self.table[i][j] = 1;
            self.table[j][i] = 1;
            self.queue.push_back((i, j));
        }
    }
    fn is_same(&self, i: usize, j: usize) -> bool {
        self.table[i][j] == 2
    }
    fn is_not_same(&self, i: usize, j: usize) -> bool {
        self.table[i][j] == 1
    }

    fn cnt_origin(&self) -> usize {
        let mut cnt = 0;
        for i in 0..self.table.len() {
            for j in 0..i {
                if self.table[i][j] == 2 {
                    cnt += 1;
                    break;
                }
            }
        }
        self.table.len() - cnt
    }

    fn process(&mut self, m: &Moves) {
        while let Some((i, j)) = self.queue.pop_front() {
            if self.is_same(i, j) {
                for k in 0..self.table.len() {
                    if self.is_same(j, k) {
                        self.set_same(i, k);
                    } else if self.is_same(i, k) {
                        self.set_same(j, k);
                    }
                    if self.is_not_same(j, k) {
                        self.set_not_same(i, k);
                    } else if self.is_not_same(i, k) {
                        self.set_not_same(j, k);
                    }
                }
                if i != m.door.len() && j != m.door.len() && m.door[i] == m.door[j] {
                    self.set_same(i + 1, j + 1);
                }
            } else if self.is_not_same(i, j) {
                for k in 0..self.table.len() {
                    if self.is_same(j, k) {
                        self.set_not_same(i, k);
                    } else if self.is_same(i, k) {
                        self.set_not_same(j, k);
                    }
                }
            }
        }
    }
}

fn dfs(list: &Vec<usize>, m: &Moves, step: usize) -> usize {
    if list.len() == 0 {
        return 0;
    }
    if list.len() == 1 {
        return 1;
    }
    let mut ans = 1;
    for t in 0..6 {
        let mut new_list = vec![vec![]; 4];
        for &i in list {
            if i + step < m.door.len() && m.door[i + step] == t {
                new_list[m.label[i + step + 1]].push(i);
            }
        }

        let mut sum = 0;
        for t2 in 0..4 {
            let res = dfs(&new_list[t2], m, step + 1);
            sum += res;
        }
        if ans < sum {
            ans = sum;
        }
    }
    return ans;
}

fn dfs2(list: &Vec<usize>, m: &Moves, step: usize, need: usize, st: &mut SameTable) {
    if need == 1 {
        for a in 0..list.len() {
            for b in a + 1..list.len() {
                st.set_same(list[a], list[b]);
                st.process(m);
            }
        }
        return;
    }
    if list.len() == 0 {
        return;
    }
    if list.len() == 1 {
        return;
    }

    for t in 0..6 {
        let mut new_list = vec![vec![]; 4];
        for &i in list {
            if i + step < m.door.len() && m.door[i + step] == t {
                new_list[m.label[i + step + 1]].push(i);
            }
        }

        for a in 0..4 {
            for b in a + 1..4 {
                for &i in &new_list[a] {
                    for &j in &new_list[b] {
                        st.set_not_same(i, j);
                        st.process(m);
                    }
                }
            }
        }

        let mut sum = 0;
        let mut ress = vec![];
        for t2 in 0..4 {
            let res = dfs(&new_list[t2], m, step + 1);
            sum += res;
            ress.push(res);
        }
        if sum == need {
            for t2 in 0..4 {
                if ress[t2] == 1 {
                    for a in 0..new_list[t2].len() {
                        for b in a + 1..new_list[t2].len() {
                            st.set_same(new_list[t2][a], new_list[t2][b]);
                            st.process(m);
                        }
                    }
                } else if ress[t2] >= 2 {
                    return dfs2(&new_list[t2], m, step + 1, ress[t2], st);
                }
            }
        }
    }
}

fn find_creek(
    list: &Vec<usize>,
    start: usize,
    now: &Vec<usize>,
    num: usize,
    st: &SameTable,
) -> Vec<usize> {
    for i in start..list.len() {
        let u = list[i];
        let mut ok = true;
        for &v in now {
            if !st.is_not_same(u, v) {
                ok = false;
                break;
            }
        }
        if ok {
            let mut next_list = now.clone();
            next_list.push(u);
            if next_list.len() == num {
                return next_list;
            }
            let res = find_creek(list, i + 1, &next_list, num, st);
            if res.len() == num {
                return res;
            }
        }
    }
    return vec![];
}

fn main() {
    let judge = get_judge_from_stdin_with(true);
    let fix_label = true;
    let use_diff = true;

    let n = judge.num_rooms();
    // 事前に与えられた explore ログを使用
    let exp = judge.explored();
    assert!(
        !exp.plans.is_empty(),
        "explored is empty; provide explores via JSON"
    );
    let plan = exp.plans[0].clone();
    let labels = exp.results[0].clone();
    let mut m = Moves {
        label: vec![],
        door: vec![],
    };
    m.label = labels.clone();
    m.door = plan.clone();

    let mut st = SameTable::new(m.door.len() + 1);

    /*
    for k in 0..2 {
        for i in 0..n {
            for j in i + 1..n {
                let a = i * 18 + 5 * k;
                let b = j * 18 + 5 * k;
                let mut ok = true;
                for k in 0..5 {
                    if m.label[a + k] != m.label[b + k] {
                        ok = false;
                    }
                }
                if ok {
                    for k in 4..5 {
                        st.set_same(a + k, b + k);
                    }
                }
            }
        }
    }
    */

    for i in 0..m.label.len() - 1 {
        for j in i + 1..m.label.len() {
            if m.label[i] != m.label[j] {
                st.set_not_same(i, j);
            }
        }
    }
    st.process(&m);

    eprint!("orig: {} / {}, ", st.cnt_origin(), labels.len());

    let mut lists = vec![vec![]; 4];
    for i in 0..m.label.len() {
        lists[m.label[i]].push(i);
    }
    for i in 0..4 {
        let mut nums = n / 4;
        if i < n % 4 {
            nums += 1;
        }
        dfs2(&lists[i], &m, 0, nums, &mut st);
    }
    st.process(&m);

    eprint!("result: {} / {}, ", st.cnt_origin(), labels.len());

    for i in 0..4 {
        let mut nums = n / 4;
        if i < n % 4 {
            nums += 1;
        }
        let res = find_creek(&lists[i], 0, &mut vec![], nums, &st);
        if res.len() == nums {
            for a in 0..res.len() {
                for b in a + 1..res.len() {
                    st.set_not_same(res[a], res[b]);
                    st.process(&m);
                }
            }

            for a in 0..lists[i].len() {
                let mut cnt = 0;
                let mut sum = 0;
                for &b in &res {
                    if !st.is_not_same(lists[i][a], b) {
                        cnt += 1;
                        sum = b;
                    }
                }
                if cnt == 1 {
                    st.is_same(lists[i][a], sum);
                }
            }
        }
    }

    eprintln!("last: {} / {}, ", st.cnt_origin(), labels.len());

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

    for i in 0..m.label.len() {
        for j in i + 1..m.label.len() {
            if st.is_not_same(i, j) {
                for u in 0..n {
                    sat.add_clause([-V[i][u], -V[j][u]]);
                }
            }
            if st.is_same(i, j) {
                for u in 0..n {
                    sat.add_clause([-V[i][u], V[j][u]]);
                    sat.add_clause([-V[j][u], V[i][u]]);
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
