#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
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

    let orig_start_label = judge.explore(&vec![vec![]])[0][0];
    let start_label = (orig_start_label + 1) % 4;

    let suffixes: Vec<Vec<Step>> = (0..36)
        .map(|i| {
            let mut s = vec![];
            for door in [i % 6, (i % 6 + i / 6 + 1) % 6] {
                s.push((Some(rng.random_range(0..4)), door));
            }
            for _ in 0..(42 - 2) {
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

    let mut cost = 0usize;

    let mut cnt = 0;
    while !queue.is_empty() {
        let paths = queue.drain(..queue.len().min(20)).collect::<Vec<_>>();
        // queue = VecDeque::new();
        assert!(cnt < 7 * n);
        cnt += 1;
        let mut batched_plans: Vec<Vec<Step>> = vec![];
        for path in paths.iter() {
            let plans = suffixes
                .iter()
                .map(|s| {
                    let mut p = path
                        .iter()
                        .enumerate()
                        .map(|(i, &door)| (if i == 0 { Some(start_label) } else { None }, door))
                        .collect::<Vec<_>>();
                    p.extend(s);
                    p
                })
                .collect::<Vec<_>>();
            batched_plans.extend(plans);
        }
        let batched_results = judge.explore(&batched_plans);
        cost += batched_plans.len() + 1;
        // for (i, path) in paths.into_iter().enumerate() {}
        for (path, results) in paths
            .into_iter()
            .zip(batched_results.chunks_exact(suffixes.len()))
        {
            // let results = judge.explore(&plans);
            // let start_index = suffixes.len() * i;
            // let stop_index = suffixes.len() * (i + 1);
            // let results = &batched_results[start_index..stop_index];
            // let plans = &batched_plans[start_index..stop_index];
            // cost += plans.len() + 1;
            let mut results = results
                .iter()
                .map(|r| r[path.len()..].to_vec())
                .collect::<Vec<_>>();
            if path.is_empty() {
                for result in results.iter_mut() {
                    // assert_eq!(result[0], orig_start_label);
                    result[0] = start_label;
                }
            }
            let room = *res_to_room.entry(results.clone()).or_insert_with(|| {
                let r = room_to_res.len();
                room_to_res.push(results.clone());
                room_to_a_path.push(path.clone());
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

    let start = 0;
    let mut rooms = room_to_res.iter().map(|r| r[0][0]).collect::<Vec<_>>();
    rooms[0] = orig_start_label;
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
