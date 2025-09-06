#![allow(
    clippy::collapsible_if,
    clippy::cast_abs_to_unsigned,
    clippy::ptr_arg,
    clippy::needless_return,
    clippy::len_zero,
    clippy::needless_range_loop
)]
#![allow(unused_imports, unused_variables, unused_mut, unused_parens)]
use clap::Parser;
use icfpc2025::judge::*;
use rand::prelude::*;
use sha1::digest::typenum::Same;
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
    let explores = judge.explored();
    let first = explores
        .first()
        .expect("explored is empty; provide explores via JSON");
    m.door = first.plans[0].clone();
    m.label = first.results[0].clone();

    //numがちゃんと検出できるか調べる
    let mut lists = vec![vec![]; 4];
    for i in 0..m.label.len() {
        lists[m.label[i]].push(i);
    }
    let mut sum = 0;
    let mut nums2 = vec![];
    for i in 0..4 {
        let res = dfs(&lists[i], &m, 0);
        sum += res;
        nums2.push(res);
    }

    let mut nums = vec![0, 0, 0, 0];
    let mut cnts = [0usize; 4];
    for i in 0..m.label.len() {
        //eprintln!("label: {}, door: {}", m.label[i], m.door[i]);
        cnts[m.label[i]] += 1;
    }

    for _ in 0..n {
        let mut best = 0.0;
        let mut id = 0;
        for i in 0..4 {
            let p = (cnts[i] as f64 + 0.01) / (nums[i] as f64 + 0.01);
            if p > best {
                best = p;
                id = i;
            }
        }
        nums[id] += 1;
    }
    //roomの数を出力
    eprintln!("nums: {:?}", nums);
    eprintln!("nums2: {:?}", nums2);

    eprintln!("sum: {} / {}", sum, n);

    if sum != n {
        eprintln!("sum != n");
        return;
    }

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
        dfs2(&lists[i], &m, 0, nums2[i], &mut st);
    }
    st.process(&m);
    eprintln!("after initial: {}", st.cnt_origin());

    let mut tekitou_merge = 0;
    while (st.cnt_origin() > n) {
        let mut made_progress = false;
        let i = rnd.random_range(0..m.label.len());
        let j = rnd.random_range(0..m.label.len());
        if i != j {
            if st.is_same(i, j) || st.is_not_same(i, j) {
                continue;
            }
            st.set_same(i, j);
            tekitou_merge += 1;
            st.process(&m);
            eprintln!("now: {}", st.cnt_origin());
        }
    }

    eprintln!("origin: {}, merge: {}", st.cnt_origin(), tekitou_merge);
}
