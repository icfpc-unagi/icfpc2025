use std::{io::Read as _, vec};

use anyhow::{Context as _, Result};
use icfpc2025::{api::Map, judge::{Guess, JsonIn}, SetMinMax as _};

fn main() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let map: Map = serde_json::from_str::<JsonIn>(input.trim()).context("invalid JSON")?.map.context("missing map")?;
    let Guess { start, rooms, graph } = map.into();
    let n = rooms.len();
    // let graph = (0..n)
    //     .map(|i| {
    //         (0..6)
    //             .map(|d| graph[i][d].0)
    //             .collect::<Vec<_>>()
    //     })
    //     .collect::<Vec<_>>();
    let graph = graph.into_iter().map(|v| v.into_iter().map(|(r, _)| r).collect::<Vec<_>>()).collect::<Vec<_>>();
    eprintln!("start = {}", start);
    eprintln!("rooms = {:?}", rooms);
    eprintln!("graph = {:?}", graph);

    let mut eq = vec![vec![true; n]; n];
    for i in 0..n {
        for j in 0..n {
            eq[i][j] = rooms[i] == rooms[j];
        }
    }

    loop {
        let mut new_eq = eq.iter().cloned().collect::<Vec<_>>();
        for i in 0..n {
            for j in 0..n {
                for d in 0..6 {
                    let ni = graph[i][d];
                    let nj = graph[j][d];
                    new_eq[i][j].setmin(eq[ni][nj]);
                }
            }
        }
        if new_eq == eq {
            break;
        }
        eq = new_eq;
    }

    let mut done = vec![false; n];
    let classes = (0..n)
        .filter_map(|i| {
            (!done[i]).then(|| {
                done[i] = true;
                let mut cls = vec![i];
                for j in i + 1..n {
                    if eq[i][j] {
                        assert!(!done[j]);
                        done[j] = true;
                        cls.push(j);
                    }
                }
                cls
            })
        })
        .collect::<Vec<_>>();
    eprintln!("classes = {:?}", classes);
    Ok(())
}
