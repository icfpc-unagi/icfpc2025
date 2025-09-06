//! # Judging and Environment Interaction
//!
//! This module provides a `Judge` trait that abstracts the interaction between a solver
//! and the problem environment (the "Aedificium"). It offers two implementations:
//!
//! - `LocalJudge`: For local testing and debugging. It can generate random problem
//!   instances or load a specific map from a file or JSON.
//! - `RemoteJudge`: For communicating with the official contest server via the `api` module.
//!
//! The `get_judge_from_stdin` function acts as a factory, creating the appropriate
//! judge instance based on command-line arguments or piped input, allowing the same
//! solver binary to be used for both local testing and remote submission.

use crate::*;
use itertools::Itertools;
use proconio::*;
use rand::prelude::*;

/// A trait abstracting the problem environment.
///
/// This allows solver logic to be written once and used against both a local
/// simulator (`LocalJudge`) and the remote contest server (`RemoteJudge`).
pub trait Judge {
    /// Returns the number of rooms in the problem.
    fn num_rooms(&self) -> usize;
    /// Returns the name of the problem.
    fn problem_name(&self) -> &str;
    /// Submits exploration plans to the judge and returns the results.
    /// The results are sequences of room signatures observed during traversal.
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>>;
    /// Submits a final map guess to the judge. Returns `true` if the guess is correct.
    fn guess(&self, out: &Guess) -> bool;
    /// Returns a log of all explorations made so far.
    fn explored(&self) -> Vec<Explored>;
    /// Sets the exploration log, useful for replaying or resuming a state.
    fn set_explored(&mut self, explored: Vec<Explored>);
}

/// Represents a solver's guess for the map's structure.
#[derive(Clone, Debug)]
pub struct Guess {
    /// The signature of each room. `rooms[i]` is the signature of room `i`.
    /// A room's signature is the number of passages connected to it.
    pub rooms: Vec<usize>,
    /// The index of the starting room.
    pub start: usize,
    /// The connections (passages) of the map. `graph[i][d]` is a tuple `(room, door)`
    /// indicating that door `d` of room `i` connects to the specified door of the other room.
    pub graph: Vec<[(usize, usize); 6]>,
}

/// A record of an exploration query and its result.
#[derive(Clone, Debug)]
pub struct Explored {
    /// The list of plans (sequences of door choices) sent in the query.
    pub plans: Vec<Vec<usize>>,
    /// The list of results (sequences of room signatures) returned by the judge.
    pub results: Vec<Vec<usize>>,
}

/// A local judge that simulates the problem environment.
///
/// It can generate random maps or be initialized with a specific map structure.
/// This is used for testing solvers without interacting with the remote server.
pub struct LocalJudge {
    problem_name: String,
    /// The signature of each room.
    rooms: Vec<usize>,
    /// The true graph of the map. `graph[i][d]` is the index of the room
    /// connected to door `d` of room `i`.
    pub graph: Vec<[usize; 6]>,
    /// The cumulative cost of explorations.
    cost: usize,
    /// A log of all explorations performed.
    explored_log: Vec<Explored>,
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
            let mut u = 0; // Start at room 0 (the fixed starting room in the problem spec)
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
        self.explored_log.push(Explored {
            plans: plans.to_vec(),
            results: ret.clone(),
        });
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
        // Basic validation of the guess structure.
        if out.rooms.len() != self.rooms.len() {
            eprintln!("!log status WA (incorrect number of rooms)");
            return false;
        }
        for i in 0..out.graph.len() {
            for door in 0..6 {
                let (i2, door2) = out.graph[i][door];
                assert_eq!(out.graph[i2][door2], (i, door), "Graph is not undirected");
            }
        }

        // Check for isomorphism between the true map and the guessed map.
        // This is done via a graph traversal (BFS-like) starting from the known
        // starting points (room 0 in the true map, `out.start` in the guess).
        let n = self.rooms.len();
        let mut dp = mat![false; n; n];
        if self.rooms[0] != out.rooms[out.start] {
            eprintln!("!log status WA (starting room signature mismatch)");
            return false;
        }
        dp[0][out.start] = true;
        let mut stack = vec![(0, out.start)];
        while let Some((u, v)) = stack.pop() {
            for door in 0..6 {
                let u2 = self.graph[u][door];
                let v2 = out.graph[v][door].0;
                // The room signatures must match for corresponding rooms.
                if self.rooms[u2] != out.rooms[v2] {
                    eprintln!("!log status WA (room signature mismatch during traversal)");
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
    fn explored(&self) -> Vec<Explored> {
        self.explored_log.clone()
    }
    fn set_explored(&mut self, explored: Vec<Explored>) {
        self.explored_log = explored;
    }
}

/// A judge that interacts with the remote contest server.
///
/// It uses the `api` module to send HTTP requests for selecting, exploring,
/// and guessing.
pub struct RemoteJudge {
    problem_name: String,
    num_rooms: usize,
    /// The cumulative cost of explorations.
    cost: usize,
    /// A log of all explorations performed.
    explored_log: Vec<Explored>,
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
        // Delegate the actual exploration to the API client.
        let ret = api::explore(plans).expect("Failed to explore").results;
        self.explored_log.push(Explored {
            plans: plans.to_vec(),
            results: ret.clone(),
        });
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
        // Convert the Guess struct into the format required by the API.
        let mut connections = vec![];
        for i in 0..out.graph.len() {
            for door in 0..6 {
                let (i2, door2) = out.graph[i][door];
                assert_eq!(out.graph[i2][door2], (i, door), "Graph is not undirected");
                // Add each edge only once to avoid duplicates.
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
        // Delegate the guess to the API client.
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
    fn explored(&self) -> Vec<Explored> {
        self.explored_log.clone()
    }
    fn set_explored(&mut self, explored: Vec<Explored>) {
        self.explored_log = explored;
    }
}

impl RemoteJudge {
    /// Creates a new `RemoteJudge` for a given problem.
    ///
    /// This function calls `api::select` to lock the problem on the server.
    pub fn new(problem_name: &str) -> Self {
        api::select(problem_name).expect("Failed to select problem");
        Self {
            problem_name: problem_name.to_string(),
            num_rooms: problems::get_problem(problem_name)
                .unwrap_or_else(|| panic!("Unknown problem: {}", problem_name))
                .size,
            cost: 0,
            explored_log: Vec::new(),
        }
    }
}

impl LocalJudge {
    /// Creates a new `LocalJudge` with a randomly generated map.
    pub fn new(problem_type: &str, num_rooms: usize, seed: u64) -> Self {
        let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
        match problem_type {
            "random" => {
                // Generate room signatures.
                let mut rooms = (0..num_rooms).map(|i| i % 4).collect_vec();
                rooms.shuffle(&mut rng);
                // Generate a random perfect matching on the set of all doors to create the graph's passages.
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
                    explored_log: Vec::new(),
                }
            }
            _ => panic!("Unknown problem type: {}", problem_type),
        }
    }

    /// Creates a new `LocalJudge` from a map structure provided in an `api::Map`.
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
            explored_log: Vec::new(),
        }
    }
}

/// Creates a `Box<dyn Judge>` by parsing configuration from standard input.
/// This allows for flexible invocation of the solver.
pub fn get_judge_from_stdin() -> Box<dyn Judge> {
    get_judge_from_stdin_with(false)
}

/// Creates a `Box<dyn Judge>` from stdin, optionally performing a random exploration first.
pub fn get_judge_from_stdin_with(explored: bool) -> Box<dyn Judge> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let s = input.trim_start();
    // If input begins with '{', treat the entire input as a single JSON object.
    // This provides a flexible way to configure the judge for local testing,
    // allowing pre-seeding of maps, exploration logs, etc.
    if s.starts_with('{') {
        #[derive(serde::Deserialize)]
        struct ExploreIn {
            plans: Vec<String>,
            results: Vec<Vec<usize>>,
        }
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
            #[serde(default)]
            explores: Option<Vec<ExploreIn>>,
        }
        let parsed: JsonIn = serde_json::from_str(s).expect("invalid JSON for json mode");

        // Helper to convert explores JSON into the internal `Explored` struct.
        fn to_explored(exps: Vec<ExploreIn>) -> Vec<Explored> {
            let mut out = Vec::with_capacity(exps.len());
            for e in exps {
                let plans = e
                    .plans
                    .into_iter()
                    .map(|p| p.chars().map(|c| (c as u8 - b'0') as usize).collect())
                    .collect();
                out.push(Explored {
                    plans,
                    results: e.results,
                });
            }
            out
        }

        let mut j: Box<dyn Judge> = match parsed.mode.as_deref() {
            Some("remote") => {
                let name = parsed
                    .problem_name
                    .as_ref()
                    .expect("problemName is required for remote mode");
                let mut jr = RemoteJudge::new(name);
                if let Some(exps) = parsed.explores {
                    jr.set_explored(to_explored(exps));
                }
                Box::new(jr)
            }
            Some("local") | None => {
                if let Some(map) = parsed.map {
                    // Create a local judge from a complete map definition.
                    Box::new(LocalJudge::new_json(parsed.problem_name, &map))
                } else if let Some(exps) = parsed.explores {
                    // Create a local judge from existing exploration results, without the true map.
                    // This is useful for "replaying" a remote session locally.
                    let explored_log = to_explored(exps);
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
                        rooms: vec![0; num_rooms], // True room signatures are unknown
                        graph: vec![[0; 6]; num_rooms], // True graph is unknown
                        cost: 0,
                        explored_log,
                    })
                } else {
                    panic!("JSON must contain either 'map' or 'explores'");
                }
            }
            Some(other) => panic!("unknown JSON mode: {}", other),
        };

        // Optionally pre-populate with a random exploration if requested and none were provided in the JSON.
        if explored && j.explored().is_empty() {
            let n = j.num_rooms();
            let mut rng = rand::thread_rng();
            let mut plan = Vec::with_capacity(18 * n);
            for _ in 0..(18 * n) {
                plan.push(rng.gen_range(0..6));
            }
            let _ = j.explore(&[plan]);
        }
        return j;
    }

    // Otherwise, parse tokens via proconio from the string.
    // This is a more traditional competitive programming input style.
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
            Box::new(LocalJudge::.new(&problem_type, num_rooms, seed))
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

    // Optionally pre-populate with a random exploration if requested.
    if explored && j.explored().is_empty() {
        let n = j.num_rooms();
        let mut rng = rand::thread_rng();
        let mut plan = Vec::with_capacity(18 * n);
        for _ in 0..(18 * n) {
            plan.push(rng.gen_range(0..6));
        }
        let _ = j.explore(&[plan]);
    }
    j
}

/// A utility function to check if a given `Guess` is consistent with past explorations.
///
/// This can be used by a solver to validate its own hypothesis against the known data.
///
/// # Arguments
/// * `guess` - The map hypothesis to check.
/// * `plans` - The exploration plans that were executed.
/// * `results` - The corresponding results received from the judge.
///
/// # Returns
/// `true` if the guess perfectly reproduces the results for all given plans.
pub fn check_explore(guess: &Guess, plans: &[Vec<usize>], results: &[Vec<usize>]) -> bool {
    assert_eq!(plans.len(), results.len());
    for (plan, result) in plans.iter().zip(results.iter()) {
        // Simulate the plan on the guessed map.
        let mut u = guess.start;
        let mut route = vec![guess.rooms[u]];
        for &door in plan {
            u = guess.graph[u][door].0;
            route.push(guess.rooms[u]);
        }
        // Check if the simulated route matches the actual result.
        if &route != result {
            eprintln!("expected: {}", result.iter().join(""));
            eprintln!("actual  : {}", route.iter().join(""));
            return false;
        }
    }
    true
}
