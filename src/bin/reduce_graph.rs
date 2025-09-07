#![allow(
    clippy::needless_range_loop,
    clippy::filter_map_bool_then, // https://github.com/rust-lang/rust-clippy/issues/11617
)]
use std::io::Read as _;

use anyhow::{Context as _, Result};
use icfpc2025::{
    SetMinMax as _, api,
    judge::{Guess, JsonIn},
};

#[derive(serde::Serialize, Debug)]
struct JsonOut {
    map: api::Map,
    permutations: Vec<Vec<Perm3>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
enum Perm3 {
    // identity
    I,
    // swap
    X,
    Y,
    Z,
    // rotate
    P,
    Q,
}

// impl<F> TryFrom<F> for Perm3
// where F: AsRef<[usize]>
impl TryFrom<&Vec<usize>> for Perm3 {
    type Error = &'static str;
    fn try_from(v: &Vec<usize>) -> Result<Self, Self::Error> {
        match v.as_slice() {
            [0] | [0, 1] | [0, 1, 2] => Ok(Perm3::I),
            [1, 0] | [1, 0, 2] => Ok(Perm3::X),
            [2, 1, 0] => Ok(Perm3::Y),
            [0, 2, 1] => Ok(Perm3::Z),
            [1, 2, 0] => Ok(Perm3::P),
            [2, 0, 1] => Ok(Perm3::Q),
            _ => Err("invalid permutation"),
        }
    }
}

impl std::ops::Neg for Perm3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        match self {
            Perm3::P => Perm3::Q,
            Perm3::Q => Perm3::P,
            _ => self,
        }
    }
}

fn fill_doors_with_perm(
    graph: &[Vec<usize>],
    permutations: &[Vec<Perm3>],
) -> Vec<[(usize, usize); 6]> {
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
                    if res[v][back_door] == (u, na)
                        && permutations[u][door] == -permutations[v][back_door]
                    {
                        res[u][door].1 = back_door;
                        res[v][back_door].1 = door;
                        ok = true;
                        break;
                    }
                }
                assert!(ok, "no back door found for {} --{}--> {}", u, door, v);
            }
        }
    }
    res
}

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

    let map: api::Map = serde_json::from_str::<JsonIn>(input.trim())
        .context("invalid JSON")?
        .map
        .context("missing map")?;
    let Guess {
        start,
        rooms,
        graph,
    } = (&map).into();
    let n = rooms.len();

    eprintln!("start = {}", start);
    eprintln!("rooms = {:?}", rooms);
    let graph = graph
        .iter()
        .map(|v| v.iter().map(|(r, _)| *r).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    eprintln!("graph = {:?}", graph);

    let mut eq = vec![vec![true; n]; n];
    for i in 0..n {
        for j in 0..n {
            eq[i][j] = rooms[i] == rooms[j];
        }
    }

    loop {
        let mut new_eq = eq.clone();
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

    let mut transitions = vec![vec![!0; 6]; m];
    let mut permutations = vec![vec![Perm3::I; 6]; m];
    for from in 0..m {
        for d in 0..6 {
            let mut to = vec![];
            let mut perm = vec![];
            for j in 0..k {
                let from_orig = classes[from][j];
                let to_orig = graph[from_orig][d];
                let (t, p) = renamed[to_orig];
                to.push(t);
                perm.push(p);
            }
            let to = from_const_vec(to);
            transitions[from][d] = to;
            permutations[from][d] = (&perm).try_into().unwrap();
        }
    }

    {
        eprint!(" ");
        for d in 0..6 {
            eprint!("    {:2}", d);
        }
        eprintln!();
    }
    for i in 0..m {
        eprint!("{:2}:", i);
        for d in 0..6 {
            eprint!(" {:2}({:?})", transitions[i][d], permutations[i][d]);
        }
        eprintln!();
    }

    let graph = fill_doors_with_perm(&transitions, &permutations);
    for i in 0..m {
        eprintln!("{:?}", graph[i]);
    }

    let guess = Guess {
        start: renamed[start].0,
        rooms: classes.iter().map(|cls| rooms[cls[0]]).collect(),
        graph,
    };
    let map = api::Map::try_from(&guess)?;
    let output = JsonOut { map, permutations };
    let json_out = serde_json::to_string(&output).unwrap();
    println!("{}", json_out);
    Ok(())
}
