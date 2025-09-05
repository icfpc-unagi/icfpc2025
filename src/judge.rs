use crate::*;
use itertools::Itertools;
use proconio::*;
use rand::prelude::*;

pub trait Judge {
    fn num_rooms(&self) -> usize;
    fn problem_name(&self) -> &str;
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>>;
    fn guess(&self, out: &Guess) -> bool;
}

pub struct Guess {
    pub rooms: Vec<usize>,
    pub start: usize,
    pub graph: Vec<[(usize, usize); 6]>,
}

pub struct LocalJudge {
    problem_name: String,
    rooms: Vec<usize>,
    graph: Vec<[usize; 6]>,
    cost: usize,
}

impl Judge for LocalJudge {
    fn num_rooms(&self) -> usize {
        self.rooms.len()
    }
    fn problem_name(&self) -> &str {
        &self.problem_name
    }
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>> {
        println!("explore {}", plans.len());
        self.cost += plans.len() + 1;
        let mut ret = vec![];
        for plan in plans {
            println!("{}", plan.iter().map(|&d| d.to_string()).join(""));
            let mut u = 0;
            let mut route = vec![self.rooms[u]];
            for &door in plan {
                assert!(door < 6);
                u = self.graph[u][door];
                route.push(self.rooms[u]);
            }
            ret.push(route);
        }
        assert!(plans.len() <= 18 * self.num_rooms());
        ret
    }
    fn guess(&self, out: &Guess) -> bool {
        println!("guess");
        println!("{}", out.rooms.iter().map(|&r| r.to_string()).join(""));
        for i in 0..out.graph.len() {
            println!(
                "{}",
                out.graph[i]
                    .iter()
                    .map(|&(r, d)| format!("{} {}", r, d))
                    .join(" ")
            );
        }
        if out.rooms.len() != self.rooms.len() {
            eprintln!("!log status WA");
            return false;
        }
        for i in 0..out.graph.len() {
            for door in 0..6 {
                assert_eq!(out.graph[i][door], (i, door), "Graph is not undirected");
            }
        }
        let n = self.rooms.len();
        let mut dp = mat![false; n; n];
        if self.rooms[0] != out.rooms[0] {
            eprintln!("!log status WA");
            return false;
        }
        dp[0][out.start] = true;
        let mut stack = vec![(0, out.start)];
        while let Some((u, v)) = stack.pop() {
            for door in 0..6 {
                let u2 = self.graph[u][door];
                let v2 = out.graph[v][door].0;
                if self.rooms[u2] != out.rooms[v2] {
                    eprintln!("!log status WA");
                    return false;
                }
                if dp[u2][v2].setmax(true) {
                    stack.push((u2, v2));
                }
            }
        }
        eprintln!("!log status AC");
        eprintln!("!log score {}", self.cost);
        true
    }
}

pub struct RemoteJudge {
    problem_name: String,
    num_rooms: usize,
}

impl Judge for RemoteJudge {
    fn num_rooms(&self) -> usize {
        self.num_rooms
    }
    fn problem_name(&self) -> &str {
        &self.problem_name
    }
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>> {
        assert!(plans.len() <= 18 * self.num_rooms);
        api::explore(plans)
            .expect("Failed to explore")
            .results
            .iter()
            .map(|r| r.iter().map(|&x| x as usize).collect())
            .collect()
    }
    fn guess(&self, out: &Guess) -> bool {
        let mut connections = vec![];
        for i in 0..out.graph.len() {
            for door in 0..6 {
                assert_eq!(out.graph[i][door], (i, door), "Graph is not undirected");
                if (i, door) <= out.graph[i][door] {
                    connections.push(api::MapConnection {
                        from: api::MapConnectionEnd { room: i, door },
                        to: api::MapConnectionEnd {
                            room: out.graph[i][door].0,
                            door: out.graph[i][door].1,
                        },
                    });
                }
            }
        }
        api::guess(&api::Map {
            rooms: out.rooms.clone(),
            starting_room: out.start as usize,
            connections,
        })
        .expect("Failed to guess")
    }
}

impl RemoteJudge {
    pub fn new(problem_name: &str) -> Self {
        api::select(problem_name).expect("Failed to select problem");
        Self {
            problem_name: problem_name.to_string(),
            num_rooms: problems::get_problem(problem_name)
                .unwrap_or_else(|| panic!("Unknown problem: {}", problem_name))
                .size,
        }
    }
}

impl LocalJudge {
    pub fn new(problem_type: &str, num_rooms: usize, seed: u64) -> Self {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
        match problem_type {
            "random" => {
                let rooms = (0..num_rooms).map(|_| rng.random_range(0..4)).collect_vec();
                let mut graph = vec![[!0; 6]; num_rooms];
                let mut list = vec![];
                for i in 0..num_rooms {
                    for door in 0..6 {
                        list.push((i, door));
                    }
                }
                list.shuffle(&mut rng);
                for i in 0..list.len() / 2 {
                    let (u1, d1) = list[2 * i];
                    let (u2, d2) = list[2 * i + 1];
                    graph[u1][d1] = u2;
                    graph[u2][d2] = u1;
                }
                Self {
                    problem_name: problem_type.to_string(),
                    rooms,
                    graph,
                    cost: 0,
                }
            }
            _ => panic!("Unknown problem type: {}", problem_type),
        }
    }
}

pub fn get_judge_from_stdin() -> Box<dyn Judge> {
    input! {
        local_remote: String,
    }
    if local_remote == "local" {
        input! {
            problem_type: String,
            num_rooms: usize,
            seed: u64,
        }
        Box::new(LocalJudge::new(&problem_type, num_rooms, seed))
    } else if local_remote == "remote" {
        input! {
            problem_name: String,
        }
        Box::new(RemoteJudge::new(&problem_name))
    } else {
        panic!("local_remote must be 'local' or 'remote'");
    }
}
