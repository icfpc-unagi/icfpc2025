use crate::*;
use itertools::Itertools;
use proconio::*;
use rand::prelude::*;

pub trait Judge {
    fn num_rooms(&self) -> usize;
    fn problem_name(&self) -> &str;
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>>;
    fn guess(&self, out: &Guess) -> bool;
    fn explored(&self) -> Explored;
    fn set_explored(&mut self, explored: Explored);
}

#[derive(Clone, Debug)]
pub struct Guess {
    pub rooms: Vec<usize>,
    pub start: usize,
    pub graph: Vec<[(usize, usize); 6]>,
}

#[derive(Clone, Debug)]
pub struct Explored {
    pub plans: Vec<Vec<usize>>,
    pub results: Vec<Vec<usize>>,
}

pub struct LocalJudge {
    problem_name: String,
    rooms: Vec<usize>,
    pub graph: Vec<[usize; 6]>,
    cost: usize,
    explored_log: Explored,
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
            assert!(plan.len() <= 18 * self.num_rooms());
        }
        for r in &ret {
            println!("{}", r.iter().join(""));
        }
        self.explored_log.plans.extend(plans.to_vec());
        self.explored_log.results.extend(ret.clone());
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
                let (i2, door2) = out.graph[i][door];
                assert_eq!(out.graph[i2][door2], (i, door), "Graph is not undirected");
            }
        }
        let n = self.rooms.len();
        let mut dp = mat![false; n; n];
        if self.rooms[0] != out.rooms[out.start] {
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
    fn explored(&self) -> Explored {
        self.explored_log.clone()
    }
    fn set_explored(&mut self, explored: Explored) {
        self.explored_log = explored;
    }
}

pub struct RemoteJudge {
    problem_name: String,
    num_rooms: usize,
    cost: usize,
    explored_log: Explored,
}

impl Judge for RemoteJudge {
    fn num_rooms(&self) -> usize {
        self.num_rooms
    }
    fn problem_name(&self) -> &str {
        &self.problem_name
    }
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>> {
        println!("explore {}", plans.len());
        self.cost += plans.len() + 1;
        for plan in plans {
            println!("{}", plan.iter().map(|&d| d.to_string()).join(""));
            assert!(plan.len() <= 18 * self.num_rooms());
        }
        let ret = api::explore(plans).expect("Failed to explore").results;
        self.explored_log.plans.extend(plans.to_vec());
        self.explored_log.results.extend(ret.clone());
        for r in &ret {
            println!("{}", r.iter().join(""));
        }
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
        let mut connections = vec![];
        for i in 0..out.graph.len() {
            for door in 0..6 {
                let (i2, door2) = out.graph[i][door];
                assert_eq!(out.graph[i2][door2], (i, door), "Graph is not undirected");
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
        let ret = api::guess(&api::Map {
            rooms: out.rooms.clone(),
            starting_room: out.start,
            connections,
        })
        .expect("Failed to guess");
        if ret {
            eprintln!("!log status AC");
            eprintln!("!log score {}", self.cost);
        } else {
            eprintln!("!log status WA");
        }
        ret
    }
    fn explored(&self) -> Explored {
        self.explored_log.clone()
    }
    fn set_explored(&mut self, explored: Explored) {
        self.explored_log = explored;
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
            cost: 0,
            explored_log: Explored {
                plans: vec![],
                results: vec![],
            },
        }
    }
}

pub fn generate_random_edges_v2(
    num_rooms: usize,
    seed: u64,
) -> Vec<((usize, usize), (usize, usize))> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut list1 = vec![];
    let mut list2 = vec![];
    for i in 0..num_rooms {
        for door in 0..6 {
            list1.push((i, door));
            list2.push((i, door));
        }
    }
    list1.shuffle(&mut rng);
    list2.shuffle(&mut rng);

    let mut used = vec![[false; 6]; num_rooms];
    let mut edges = vec![];

    let mut i2 = 0;
    for i1 in 0..list1.len() {
        let (u1, d1) = list1[i1];
        if used[u1][d1] {
            continue;
        }
        while let (u2, d2) = list2[i2]
            && used[u2][d2]
        {
            i2 += 1;
        }
        let (u2, d2) = list2[i2];
        i2 += 1;

        edges.push(((u1, d1), (u2, d2)));
        used[u1][d1] = true;
        used[u2][d2] = true;
    }

    edges
}

impl LocalJudge {
    pub fn new(problem_type: &str, num_rooms: usize, seed: u64) -> Self {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
        match problem_type {
            "random" => {
                let mut rooms = (0..num_rooms).map(|i| i % 4).collect_vec();
                rooms.shuffle(&mut rng);
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
                    explored_log: Explored {
                        plans: vec![],
                        results: vec![],
                    },
                }
            }
            "random2" => {
                let mut rooms = (0..num_rooms).map(|i| i % 4).collect_vec();
                rooms.shuffle(&mut rng);
                let edges = generate_random_edges_v2(num_rooms, seed);
                let mut graph = vec![[!0; 6]; num_rooms];
                for ((u1, d1), (u2, d2)) in edges {
                    if (u1 == u2) && (d1 == d2) {
                        eprintln!("Self-loop: {} {}", u1, d1);
                    }

                    graph[u1][d1] = u2;
                    graph[u2][d2] = u1;
                }
                Self {
                    problem_name: problem_type.to_string(),
                    rooms,
                    graph,
                    cost: 0,
                    explored_log: Vec::new(),
                }
            }
            _ => panic!("Unknown problem type: {}", problem_type),
        }
    }

    pub fn new_json(problem_name: Option<String>, map: &crate::api::Map) -> Self {
        let n = map.rooms.len();
        let mut graph = vec![[0usize; 6]; n];
        for c in &map.connections {
            let fr = &c.from;
            let to = &c.to;
            if fr.room < n && fr.door < 6 && to.room < n && to.door < 6 {
                graph[fr.room][fr.door] = to.room;
                graph[to.room][to.door] = fr.room;
            }
        }
        Self {
            problem_name: problem_name.unwrap_or_else(|| "json".to_string()),
            rooms: map.rooms.clone(),
            graph,
            cost: 0,
            explored_log: Explored {
                plans: vec![],
                results: vec![],
            },
        }
    }
}

pub fn get_judge_from_stdin() -> Box<dyn Judge> {
    get_judge_from_stdin_with(false)
}

pub fn get_judge_from_stdin_with(explored: bool) -> Box<dyn Judge> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let s = input.trim_start();
    // If input begins with '{', treat entire input as JSON
    if s.starts_with('{') {
        #[derive(serde::Deserialize)]
        struct JsonIn {
            #[serde(default)]
            mode: Option<String>,
            #[serde(rename = "problemName")]
            #[serde(default)]
            problem_name: Option<String>,
            #[serde(rename = "numRooms")]
            #[serde(default)]
            num_rooms: Option<usize>,
            #[serde(default)]
            map: Option<crate::api::Map>,
            // New JSON format: top-level single explore
            #[serde(default)]
            plans: Option<Vec<String>>, // e.g., ["0123"]
            #[serde(default)]
            results: Option<Vec<Vec<usize>>>,
        }
        let parsed: JsonIn = serde_json::from_str(s).expect("invalid JSON for json mode");

        // Helper for new single-explore format: (plans, results) at top level
        fn single_to_explored(plans: Vec<String>, results: Vec<Vec<usize>>) -> Explored {
            let plans_parsed = plans
                .into_iter()
                .map(|p| p.chars().map(|c| (c as u8 - b'0') as usize).collect())
                .collect::<Vec<Vec<usize>>>();
            Explored {
                plans: plans_parsed,
                results,
            }
        }

        let mut j: Box<dyn Judge> = match parsed.mode.as_deref() {
            Some("remote") => {
                let name = parsed
                    .problem_name
                    .as_ref()
                    .expect("problemName is required for remote mode");
                let mut jr = RemoteJudge::new(name);
                if let (Some(plans), Some(results)) =
                    (parsed.plans.as_ref(), parsed.results.as_ref())
                {
                    jr.set_explored(single_to_explored(plans.clone(), results.clone()));
                }
                Box::new(jr)
            }
            Some("local") | None => {
                if let Some(map) = parsed.map {
                    Box::new(LocalJudge::new_json(parsed.problem_name, &map))
                } else if let (Some(plans), Some(results)) = (parsed.plans, parsed.results) {
                    let explored_log = single_to_explored(plans, results);
                    let num_rooms = if let Some(n) = parsed.num_rooms {
                        n
                    } else if let Some(ref name) = parsed.problem_name {
                        problems::get_problem(name.as_str())
                            .map(|p| p.size)
                            .expect("numRooms missing and unknown problemName")
                    } else {
                        panic!("numRooms missing and problemName not provided");
                    };
                    Box::new(LocalJudge {
                        problem_name: parsed.problem_name.unwrap_or_else(|| "json".to_string()),
                        rooms: vec![0; num_rooms],
                        graph: vec![[0; 6]; num_rooms],
                        cost: 0,
                        explored_log,
                    })
                } else {
                    panic!("JSON must contain either 'map' or ('plans' & 'results')");
                }
            }
            Some(other) => panic!("unknown JSON mode: {}", other),
        };

        // Optionally pre-populate with a random explore if requested and none provided
        if explored && j.explored().plans.is_empty() {
            let n = j.num_rooms();
            let mut rng = rand::rng();
            let mut plan = Vec::with_capacity(18 * n);
            for _ in 0..(18 * n) {
                plan.push(rng.random_range(0..6));
            }
            let _ = j.explore(&[plan]);
        }
        return j;
    }

    // Otherwise, parse tokens via proconio from OnceSource
    use proconio::source::once::OnceSource;
    let mut src = OnceSource::from(s);
    input! { from &mut src, mode: String }
    let mut j: Box<dyn Judge> = match mode.as_str() {
        "local" => {
            input! {
                from &mut src,
                problem_type: String,
                num_rooms: usize,
                seed: u64,
            }
            Box::new(LocalJudge::new(&problem_type, num_rooms, seed))
        }
        "remote" => {
            input! {
                from &mut src,
                problem_name: String,
            }
            Box::new(RemoteJudge::new(&problem_name))
        }
        _ => panic!("local_remote must be 'local' or 'remote'"),
    };

    if explored && j.explored().plans.is_empty() {
        let n = j.num_rooms();
        let mut rng = rand::rng();
        let mut plan = Vec::with_capacity(18 * n);
        for _ in 0..(18 * n) {
            plan.push(rng.random_range(0..6));
        }
        let _ = j.explore(&[plan]);
    }
    j
}

pub fn check_explore(guess: &Guess, plans: &[Vec<usize>], results: &[Vec<usize>]) -> bool {
    assert_eq!(plans.len(), results.len());
    for (plan, result) in plans.iter().zip(results.iter()) {
        let mut u = guess.start;
        let mut route = vec![guess.rooms[u]];
        for &door in plan {
            u = guess.graph[u][door].0;
            route.push(guess.rooms[u]);
        }
        if &route != result {
            eprintln!("expected: {}", result.iter().join(""));
            eprintln!("actual  : {}", route.iter().join(""));
            return false;
        }
    }
    true
}
