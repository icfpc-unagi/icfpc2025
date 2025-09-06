#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::{judge::*, *};
use itertools::Itertools;
use rand::prelude::*;

fn main() {
    let mut rng = rand_pcg::Pcg64Mcg::seed_from_u64(849328);
    let judge = get_judge_from_stdin_with(true);
    let n = judge.num_rooms();
    let explored = judge.explored();
    assert!(
        !explored.plans.is_empty(),
        "explored is empty; provide explores via JSON"
    );
    let doors: Vec<usize> = explored.plans[0].iter().map(|&(_, d)| d).collect();
    let labels = explored.results[0].clone();
    let mut guess = Guess {
        rooms: vec![!0; n],
        start: 0,
        graph: vec![[(!0, !0); 6]; n],
    };
    guess.rooms[0] = labels[0];
    let mut diff = mat![false; labels.len(); labels.len()];
    loop {
        let bk = diff.clone();
        for i in 0..labels.len() {
            for j in i + 1..labels.len() {
                if labels[i] != labels[j] {
                    diff[i][j] = true;
                    diff[j][i] = true;
                } else if j < doors.len() && doors[i] == doors[j] && diff[i + 1][j + 1] {
                    diff[i][j] = true;
                    diff[j][i] = true;
                }
            }
        }
        if bk == diff {
            break;
        }
    }
    for i in 0..labels.len() {
        eprintln!(
            "{}",
            diff[i].iter().map(|&b| if b { '1' } else { '0' }).join("")
        );
    }
    let mut group = vec![0; labels.len()];
    for i in 0..labels.len() {
        group[i] = rng.random_range(0..n);
    }
    // let mut group = greedy(&labels, &diff);
    let mut crt = eval(n, &doors, &diff, &group);
    eprintln!("{:.3}: {}", get_time(), crt);
    let mut best = crt;
    while crt > 0 {
        let temp = 0.1;
        let i = rng.random_range(0..labels.len());
        let g = rng.random_range(0..n);
        let bk = group[i];
        group[i] = g;
        let next = eval(n, &doors, &diff, &group);
        if next <= crt || rng.random_bool(((crt - next) as f64 / temp).exp()) {
            crt = next;
        } else {
            group[i] = bk;
        }
        if best.setmin(crt) {
            eprintln!("{:.3}: {}", get_time(), best);
        }
    }
    let guess = get_guess(n, &doors, &labels, &group);
    assert!(check_explore(
        &guess,
        &vec![doors.clone()],
        &vec![labels.clone()]
    ));
    judge.guess(&guess);
}

fn eval(n: usize, doors: &[usize], diff: &Vec<Vec<bool>>, group: &[usize]) -> i64 {
    let mut cost = 0;
    for i in 0..group.len() {
        for j in i + 1..group.len() {
            if group[i] == group[j] && diff[i][j] {
                cost += 1;
            }
        }
    }
    let mut g = vec![[!0; 6]; n];
    for i in 0..doors.len() {
        if g[group[i]][doors[i]] != !0 && g[group[i]][doors[i]] != group[i + 1] {
            cost += 1;
        }
        g[group[i]][doors[i]] = group[i + 1];
    }
    let mut deg = mat![0; n; n];
    let mut free = vec![0; n];
    for i in 0..n {
        for d in 0..6 {
            if g[i][d] != !0 {
                deg[i][g[i][d]] += 1;
            } else {
                free[i] += 1;
            }
        }
    }
    for i in 0..n {
        for j in 0..n {
            if deg[i][j].max(deg[j][i]) > (deg[i][j] + free[i]).min(deg[j][i] + free[j]) {
                cost += 1;
            } else {
                // if deg[i][j] > deg[j][i] {
                //     free[j] -= deg[i][j] - deg[j][i];
                // } else if deg[j][i] > deg[i][j] {
                //     free[i] -= deg[j][i] - deg[i][j];
                // }
            }
        }
    }
    cost
}

fn get_guess(n: usize, doors: &[usize], labels: &[usize], group: &[usize]) -> Guess {
    let mut g = vec![[!0; 6]; n];
    for i in 0..doors.len() {
        assert!(g[group[i]][doors[i]] == !0 || g[group[i]][doors[i]] == group[i + 1]);
        g[group[i]][doors[i]] = group[i + 1];
    }
    let mut rooms = vec![0; n];
    for i in 0..group.len() {
        rooms[group[i]] = labels[i];
    }
    let mut deg = mat![0; n; n];
    for i in 0..n {
        for d in 0..6 {
            if g[i][d] != !0 {
                deg[i][g[i][d]] += 1;
            }
        }
    }
    for i in 0..n {
        for j in 0..n {
            while deg[i][j] < deg[j][i] {
                let d = (0..6).find(|&d| g[i][d] == !0).unwrap();
                g[i][d] = j;
                deg[i][j] += 1;
            }
        }
    }
    let mut graph = vec![[(!0, !0); 6]; n];
    for i in 0..n {
        for d in 0..6 {
            if g[i][d] >= i {
                let j = g[i][d];
                let d2 = (0..6)
                    .find(|&d2| g[j][d2] == i && graph[j][d2].0 == !0)
                    .unwrap();
                graph[i][d] = (j, d2);
                graph[j][d2] = (i, d);
            }
        }
    }
    Guess {
        start: group[0],
        rooms,
        graph,
    }
}

pub fn greedy(labels: &[usize], diff: &[Vec<bool>]) -> Vec<usize> {
    let mut groups = vec![(vec![0], diff[0].clone())];
    let mut group = vec![!0; labels.len()];
    group[0] = 0;
    for _ in 1..labels.len() {
        if let Some(i) =
            (0..labels.len()).find(|&i| group[i] == !0 && groups.iter().all(|(_, d)| d[i]))
        {
            groups.push((vec![i], diff[i].clone()));
            group[i] = groups.len() - 1;
            continue;
        }
        let mut min = (i32::MAX, !0, !0);
        for i in 0..labels.len() {
            if group[i] != !0 {
                continue;
            }
            for g in 0..groups.len() {
                if groups[g].1[i] {
                    continue;
                }
                let mut c = 0;
                for j in 0..labels.len() {
                    if j != i && group[j] == !0 && !groups[g].1[j] && diff[i][j] {
                        c += 1;
                    }
                }
                min.setmin((c, i, g));
            }
        }
        let (_, i, g) = min;
        groups[g].0.push(i);
        for j in 0..labels.len() {
            groups[g].1[j] |= diff[i][j];
        }
        group[i] = g;
    }
    group
}

pub fn get_time() -> f64 {
    static mut STIME: f64 = -1.0;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let ms = t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9;
    unsafe {
        if STIME < 0.0 {
            STIME = ms;
        }
        ms - STIME
    }
}
