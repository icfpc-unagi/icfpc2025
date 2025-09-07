#![allow(
    clippy::needless_range_loop,
    clippy::filter_map_bool_then, // https://github.com/rust-lang/rust-clippy/issues/11617
)]
use std::{io::Read as _, vec};

use anyhow::{Context as _, Result};
use icfpc2025::{
    SetMinMax as _,
    api::Map,
    judge::{Guess, JsonIn},
};

fn from_const_vec<T>(v: Vec<T>) -> T
where
    T: PartialEq + std::fmt::Debug,
{
    assert!(!v.is_empty());
    let mut v = v.into_iter();
    let first = v.next().unwrap();
    for x in v {
        assert_eq!(&x, &first);
    }
    first
}

fn main() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let map: Map = serde_json::from_str::<JsonIn>(input.trim())
        .context("invalid JSON")?
        .map
        .context("missing map")?;
    let Guess {
        start,
        rooms,
        graph,
    } = map.into();
    let n = rooms.len();

    eprintln!("start = {}", start);
    eprintln!("rooms = {:?}", rooms);
    let transition = graph
        .iter()
        .map(|v| v.iter().map(|(r, _)| *r).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    eprintln!("transition = {:?}", transition);

    let mut eq = vec![vec![true; n]; n];
    for i in 0..n {
        for j in 0..n {
            eq[i][j] = rooms[i] == rooms[j];
        }
    }

    loop {
        let mut new_eq = eq.iter().map(|row| row.to_vec()).collect::<Vec<_>>();
        for i in 0..n {
            for j in 0..n {
                for d in 0..6 {
                    let ni = transition[i][d];
                    let nj = transition[j][d];
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

    let mut renamed = vec![(!0, !0); n];
    for (i, cls) in classes.iter().enumerate() {
        for (j, &v) in cls.iter().enumerate() {
            renamed[v] = (i, j);
        }
    }

    let m = classes.len();
    let k = classes[0].len();
    assert_eq!(m * k, n);
    assert!(k <= 3);

    let mut new_graph = vec![vec![(!0, !0); 6]; m];
    let mut permutations = vec![vec![vec![]; 6]; m];
    for from in 0..m {
        for d in 0..6 {
            let mut to = vec![];
            let mut back_d = vec![];
            let mut perm = vec![];
            for j in 0..k {
                let (to_orig, b) = graph[classes[from][j]][d];
                let (t, p) = renamed[to_orig];
                to.push(t);
                back_d.push(b);
                perm.push(p);
            }
            let to = from_const_vec(to);
            let back_d = from_const_vec(back_d);
            new_graph[from][d] = (to, back_d);
            permutations[from][d] = perm;
        }
    }

    for (i, g) in new_graph.iter().enumerate() {
        eprint!("{}:", i);
        for d in 0..6 {
            let (to, back_d) = g[d];
            eprint!(" {}->{}({})", d, to, back_d);
            eprintln!();
        }
    }
    Ok(())
}
