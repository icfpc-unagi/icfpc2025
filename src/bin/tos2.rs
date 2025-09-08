#![allow(unused_variables)]
use std::collections::HashMap;

use icfpc2025::judge::*;

// struct Node {
//     parent: usize,
//     children: Vec<usize>,
// }

fn main() {
    let mut judge = get_judge_from_stdin_with(false);
    let n = judge.num_rooms();

    let mut room_to_label = vec![];
    let mut path_to_room: HashMap<Vec<usize>, usize> = HashMap::new();
    let mut to_visit = vec![];

    {
        let path = vec![];
        let room = room_to_label.len();
        path_to_room.insert(path.clone(), room);
        room_to_label.push(judge.explore(&[vec![]])[0][0]);
        to_visit.push(path);
    }

    let pairs: Vec<(usize, usize)> = (0..4)
        .flat_map(|i| (1..4).map(move |j| (i, (i + j) % 4)))
        .collect();
    while let Some(path) = to_visit.pop() {
        let k = path.len();
        assert!(path.len() <= pairs.len());
        let prefix_a = path
            .iter()
            .zip(&pairs)
            .map(|(&d, &(a, _))| (Some(a), d))
            .collect::<Vec<_>>();
        let prefix_b = path
            .iter()
            .zip(&pairs)
            .map(|(&d, &(_, b))| (Some(b), d))
            .collect::<Vec<_>>();
        // let (a, b) = pairs[path.len()];
        // let plans_a = (0..6).map(|d| {
        //     let mut p = prefix_a.clone();
        //     p.push((Some(a), d));
        //     p
        // }).collect::<Vec<_>>();
        // let plans_b = (0..6).map(|d| {
        //     let mut p = prefix_b.clone();
        //     p.push((Some(b), d));
        //     p
        // }).collect::<Vec<_>>();
        // let plans = [plans_a, plans_b].concat();
        // let results = judge.explore(&plans);
        // let results_a = &results[..6];
        // let results_b = &results[6..];
        let [result_a, result_b] = judge.explore(&[prefix_a, prefix_b]).try_into().unwrap();
        if result_a[k] == result_b[k] {}
    }
}
