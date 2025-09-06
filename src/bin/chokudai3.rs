#![allow(
    clippy::collapsible_if,
    clippy::cast_abs_to_unsigned,
    clippy::ptr_arg,
    clippy::needless_return,
    clippy::len_zero,
    clippy::needless_range_loop
)]
#![allow(unused_variables, unused_mut, dead_code)]
use clap::Parser;
use icfpc2025::judge::*;
use rand::prelude::*;
use std::collections::VecDeque;

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
        if self.table[i][j] == 0 {
            //eprintln!("set_same: {}, {}", i, j);
            self.table[i][j] = 2;
            self.table[j][i] = 2;
            self.queue.push_back((i, j));
        }
    }

    fn set_not_same(&mut self, i: usize, j: usize) {
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
                if i != 0 && j != 0 && m.door[i - 1] == m.door[j - 1] {
                    self.set_not_same(i - 1, j - 1);
                }
            }
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'i',
        long = "input",
        help = "Read input from file instead of stdin"
    )]
    input: Option<String>,
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
    if step == 1 {
        for a in 0..list.len() {
            for b in a + 1..list.len() {
                st.set_same(list[a], list[b]);
                st.process(m);
            }
        }
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

fn main() {
    let mut judge = get_judge_from_stdin_with(true);
    let n = judge.num_rooms();
    let mut m = Moves {
        label: vec![],
        door: vec![],
    };
    let mut rnd = rand::rng();
    // 事前に与えられた explore ログを使用
    let exp = judge.explored();
    assert!(
        !exp.plans.is_empty(),
        "explored is empty; provide explores via JSON"
    );
    m.door = exp.plans[0].clone();
    m.label = exp.results[0].clone();

    //推測を行う
    //4グループの個数を適当に分ける
    let mut lists = vec![vec![]; 4];
    for i in 0..m.label.len() {
        lists[m.label[i]].push(i);
    }

    let mut nums = vec![];
    let mut sum = 0;
    for i in 0..4 {
        let res = dfs(&lists[i], &m, 0);
        nums.push(res);
        sum += res;
    }

    let mut cnts = [0usize; 4];
    for i in 0..m.door.len() {
        cnts[m.label[i]] += 1;
    }
    //頻度ごとに割り当てる
    for _ in sum..n {
        let mut best = 0.0;
        let mut id = 0;
        for i in 0..4 {
            let p = (cnts[i] as f64) / (nums[i] as f64 + 2.0);
            if p > best {
                best = p;
                id = i;
            }
        }
        nums[id] += 1;
    }
    //roomの数を出力
    eprintln!("nums: {:?}", nums);

    let mut st = SameTable::new(m.door.len() + 1);
    for i in 0..m.door.len() {
        for j in 0..m.door.len() + 1 {
            if m.label[i] != m.label[j] {
                st.set_not_same(i, j);
            }
        }
    }
    st.process(&m);

    for i in 0..4 {
        dfs2(&lists[i], &m, 0, nums[i], &mut st);
    }
    st.process(&m);
    eprintln!("after initial: {} / {}", st.cnt_origin(), st.table.len());

    let mut label_start = vec![];
    label_start.push(0);
    for i in 0..3 {
        label_start.push(label_start[i] + nums[i]);
    }

    let mut label_id = vec![0; n];
    for i in 0..4 {
        for j in 0..nums[i] {
            label_id[label_start[i] + j] = i;
        }
    }

    let mut edges = vec![vec![(0, 0); 6]; n];
    let mut array = vec![];

    for i in 0..n {
        for j in 0..6 {
            array.push((i, j as usize));
        }
    }

    let mut now_p = 0;
    while now_p < array.len() {
        let target = rnd.random_range(now_p..array.len());
        if target == now_p {
            edges[array[target].0][array[target].1] = array[target];
            now_p += 1;
        } else {
            edges[array[target].0][array[target].1] = array[now_p];
            edges[array[now_p].0][array[now_p].1] = array[target];
            array[target] = array[now_p + 1];
            now_p += 2;
        }
    }

    loop {
        //ランダムにlabelを割り当てる
        let mut rng = rand::rng();

        let mut loop_cnt = 0;
        let mut wrong = error_check(&edges, &m, n, &label_id);
        let mut best_wrong = wrong;
        let mut not_update = 0;
        let mut best_edge = edges.clone();

        eprintln!("initial_wrong: {}", best_wrong);

        if best_wrong == 0 {
            //eprintln!("find initial");
            break;
        }

        loop {
            loop_cnt += 1;
            let mut new_edges = edges.clone();
            let c = rng.random_range(0..(n * 6));

            //2つの辺をランダムに選んで繋ぎ変える
            let u1 = c / 6;
            let d1 = c % 6;
            let c2 = rng.random_range(0..(n * 6));
            let u2 = c2 / 6;
            let d2 = c2 % 6;
            if u1 == u2 || new_edges[u1][d1] == (u2, d2) || new_edges[u2][d2] == (u1, d1) {
                continue;
            }
            let v1 = new_edges[u1][d1];
            let v2 = new_edges[u2][d2];
            new_edges[u1][d1] = v2;
            new_edges[u2][d2] = v1;
            new_edges[v1.0][v1.1] = (u2, d2);
            new_edges[v2.0][v2.1] = (u1, d1);

            let new_wrong = error_check(&new_edges, &m, n, &label_id);
            if new_wrong <= wrong || rnd.random_bool(0.03) {
                if new_wrong < best_wrong {
                    eprintln!("loop_cnt: {}, wrong: {}", loop_cnt, new_wrong);
                    best_wrong = new_wrong;
                    best_edge = new_edges.clone();
                    not_update = 0;
                }
                wrong = new_wrong;
                edges = new_edges.clone();
            } else {
                not_update += 1;
            }

            if best_wrong == 0 {
                eprintln!("find in loop");
                break;
            }

            if loop_cnt >= 50000 {
                break;
            }
        }

        if best_wrong != 0 {
            eprintln!("best_wrong != 0: {}", best_wrong);
            continue;
        }

        let mut out = Guess {
            rooms: vec![0; n],
            start: label_start[m.label[0]],
            graph: vec![[(0, 0); 6]; n],
        };
        let mut room_label_num = 0;
        for i in 0..n {
            while room_label_num < 3 && label_start[room_label_num + 1] <= i {
                room_label_num += 1;
            }
            out.rooms[i] = room_label_num;
        }
        for i in 0..n {
            for j in 0..6 {
                out.graph[i][j] = best_edge[i][j];
            }
        }
        judge.guess(&out);
        break;
    }
}

fn error_check(edges: &Vec<Vec<(usize, usize)>>, m: &Moves, n: usize, label_id: &[usize]) -> usize {
    let mut wrong = 0;
    let ng = 999999;
    let mm = m.label.len();
    let mut dp = vec![vec![ng; n]; mm];
    let ng_type = 100;
    for i in 0..n {
        if label_id[i] == m.label[0] {
            dp[0][i] = 0;
        } else {
            dp[0][i] = ng_type;
        }
    }

    for t in 0..mm - 1 {
        for i in 0..n {
            for j in 0..n {
                let mut cost = dp[t][i];
                //eprintln!("t: {}, i: {}, j: {}, cost: {}", t, i, j, cost);
                if edges[i][m.door[t]].0 != j {
                    cost += 1;
                }
                if label_id[j] != m.label[t + 1] {
                    cost += ng_type;
                }
                if dp[t + 1][j] > cost {
                    dp[t + 1][j] = cost;
                }
            }
        }
    }

    let mut wrong = ng;
    for j in 0..n {
        if dp[mm - 1][j] < wrong {
            wrong = dp[mm - 1][j];
        }
    }

    wrong
}

//toを使ってansを作ってみた時に上手く行くかチェックする
fn to_check(
    ans: &[usize],
    label_id: &[usize],
    to: &Vec<Vec<usize>>,
    m: &Moves,
) -> (usize, Vec<usize>) {
    let mut ret_ans = vec![0; m.label.len()];
    let mut now = ans[0];
    let mut wrong = 0;
    ret_ans[0] = now;
    for i in 0..m.door.len() {
        let next = to[now][m.door[i]];
        if label_id[next] != m.label[i + 1] {
            wrong += 1;
        }
        now = next;
        ret_ans[i + 1] = now;
    }
    (wrong, ret_ans)
}
