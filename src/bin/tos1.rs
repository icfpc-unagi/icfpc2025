use std::collections::{HashMap, VecDeque};

use icfpc2025::judge::*;
use rand::prelude::*;

fn fill_doors(graph: &[Vec<usize>]) -> Vec<[(usize, usize); 6]> {
    let na = usize::MAX;
    let mut res = vec![[(na, na); 6]; graph.len()];
    for (u, edges) in graph.iter().enumerate() {
        for (door, &v) in edges.iter().enumerate() {
            res[u][door].0 = v;
        }
    }
    for (u, edges) in graph.iter().enumerate() {
        for (door, &v) in edges.iter().enumerate() {
            if res[u][door].1 == na {
                let mut ok = false;
                for back_door in 0..6 {
                    if res[v][back_door] == (u, na) {
                        res[u][door].1 = back_door;
                        res[v][back_door].1 = door;
                        ok = true;
                        break;
                    }
                }
                assert!(ok);
            }
        }
    }
    res
}

fn main() {
    let senpuku = false;
    let mut judge = get_judge_from_stdin_with(false);
    let n = judge.num_rooms();

    let mut rng = rand::rng();

    let mut pairs: Vec<(usize, usize)> = (0..4)
        .flat_map(|i| (1..4).map(move |j| (i, (i + j) % 4)))
        .collect();
    pairs.shuffle(&mut rng);
    let pairs = pairs;
    let mut prefix_a = vec![];
    let mut prefix_b = vec![];
    for &(a, b) in pairs.iter() {
        let door = rng.random_range(0..6);
        prefix_a.push((Some(a), door));
        prefix_b.push((Some(b), door));
    }
    let prefix_a = prefix_a;
    let prefix_b = prefix_b;
    // let prefixes = vec![prefix_a, prefix_b];

    let prefix_len = pairs.len();
    let suffix_len = 42.min(4 * n - prefix_len);
    // assert_eq!(prefix_len, 12);

    let suffixes: Vec<Vec<Step>> = (0..24) // TODO: tune
        .map(|i| {
            let mut s = vec![];
            for door in [i % 6, (i % 6 + i / 6 + 1) % 6] {
                s.push((Some(rng.random_range(0..4)), door));
            }
            for _ in 2..suffix_len {
                s.push((Some(rng.random_range(0..4)), rng.random_range(0..6)));
            }
            s
        })
        .collect::<Vec<_>>();

    let mut queue = VecDeque::new();
    queue.push_back(vec![]);

    let mut path_to_room: HashMap<Vec<usize>, usize> = HashMap::new();
    let mut res_to_room: HashMap<Vec<Vec<usize>>, usize> = HashMap::new();
    let mut room_to_res: Vec<Vec<Vec<usize>>> = vec![];
    let mut room_to_a_path: Vec<Vec<usize>> = vec![];
    let mut room_to_label = vec![];

    let mut start = usize::MAX;
    let mut start_label = pairs[0];
    // let mut orig_labels = vec![];
    let mut orig_labels = HashMap::new();
    orig_labels.extend((0..4).map(|i| ((i, i), i)));

    let mut cost = 0usize;

    let mut cnt = 0;
    let max_batch_size = 20; // 1 to debug locally
    while !queue.is_empty() {
        let paths = queue
            .drain(..queue.len().min(max_batch_size))
            .collect::<Vec<_>>();
        // queue = VecDeque::new();
        assert!(cnt < 7 * n);
        cnt += 1;
        let mut batched_plans: Vec<Vec<Step>> = vec![];
        for path in paths.iter() {
            let noop_path = path.iter().map(|&d| (None, d)).collect::<Vec<_>>();
            let mut plans = vec![];
            for suffix in &suffixes {
                for prefix in [&prefix_a, &prefix_b] {
                    let mut p = prefix.clone();
                    p.extend(noop_path.iter());
                    p.extend(suffix);
                    plans.push(p);
                }
            }
            batched_plans.extend(plans);
        }
        let batched_results = judge.explore(&batched_plans);
        cost += batched_plans.len() + 1;
        // for (i, path) in paths.into_iter().enumerate() {}
        for (path, results) in paths
            .into_iter()
            .zip(batched_results.chunks_exact(suffixes.len() * 2))
        {
            if path.is_empty() {
                // first iter
                let results_a = results[0][..prefix_len].to_vec();
                let results_b = results[1][..prefix_len].to_vec();

                for (i, label_pair) in results_a.into_iter().zip(results_b).enumerate() {
                    // let (a, b) = label_pair;
                    // let orig_label = if a == b {
                    //     a;
                    // } else {
                    //     let j = (0..i).find(|&j| pairs[j] == label_pair).unwrap();
                    //     orig_labels[j]
                    // };
                    orig_labels.insert(pairs[i], orig_labels[&label_pair]);
                    if label_pair == start_label {
                        start_label = pairs[i];
                    }
                }
            }
            let results = results
                .iter()
                .map(|r| r[(prefix_len + path.len())..].to_vec())
                .collect::<Vec<_>>();
            let room = *res_to_room.entry(results.clone()).or_insert_with(|| {
                let r = room_to_res.len();
                room_to_res.push(results.clone());
                room_to_a_path.push(path.clone());
                let label_pair = (results[0][0], results[1][0]);
                room_to_label.push(orig_labels[&label_pair]);
                if label_pair == start_label {
                    eprintln!("start room: {}", r);
                    assert_eq!(start, usize::MAX);
                    start = r;
                }
                for door in 0..6 {
                    let mut p = path.clone();
                    p.push(door);
                    queue.push_back(p);
                }
                r
            });
            path_to_room.insert(path.clone(), room);
            eprintln!(
                "{} {}",
                path.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(""),
                room
            );
        }
    }

    if senpuku {
        while cost < 88999 {
            judge.explore(&vec![vec![]; 10000]);
            cost += 10001;
        }
        judge.explore(&vec![vec![]; 100000 - cost - 1]);
    }

    // for (room, res) in room_to_res.iter().enumerate() {
    //     eprintln!("room {}:", room);
    //     for r in res.iter() {
    //         eprintln!(
    //             "{}",
    //             r.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("")
    //         );
    //     }
    //     eprintln!();
    // }

    // let start = 0;
    let rooms = room_to_label;
    eprintln!("start: {}", start);
    eprintln!("rooms: {:?}", rooms);
    let graph: Vec<Vec<usize>> = room_to_a_path
        .iter()
        .map(|path| {
            (0..6)
                .map(|door| {
                    let mut p = path.clone();
                    p.push(door);
                    *path_to_room.get(&p).unwrap()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    eprintln!("graph: {:?}", graph);
    let graph = fill_doors(&graph);
    judge.guess(&Guess {
        start,
        rooms,
        graph,
    });
}
