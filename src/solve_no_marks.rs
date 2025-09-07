#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]

use itertools::Itertools;
use std::path::Path;

use crate::{
    judge::{Guess, check_explore},
    mat,
};

// ----------------------------- CNF utilities -----------------------------

pub struct Counter {
    cnt: i32,
}
impl Counter {
    fn new() -> Self {
        Self { cnt: 0 }
    }
    #[inline]
    fn next(&mut self) -> i32 {
        self.cnt += 1;
        self.cnt
    }
}

const AMO_PAIRWISE_THRESHOLD: usize = 6;

pub fn amo_pairwise(cnf: &mut Cnf, xs: &[i32]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            cnf.clause([-xs[i], -xs[j]]);
        }
    }
}

pub fn choose_one(cnf: &mut Cnf, xs: &[i32], id: &mut Counter) {}

pub struct Cnf {
    pub sat: cadical::Solver,
    id: Counter,
    buf: Vec<i32>,
    clauses: Vec<Vec<i32>>,
}

impl Cnf {
    pub fn new() -> Self {
        Self {
            sat: cadical::Solver::with_config("sat").unwrap(),
            id: Counter::new(),
            buf: Vec::with_capacity(128),
            clauses: vec![],
        }
    }
    #[inline]
    pub fn var(&mut self) -> i32 {
        self.id.next()
    }
    #[inline]
    pub fn clause<I: IntoIterator<Item = i32>>(&mut self, lits: I) {
        let lits: Vec<i32> = lits.into_iter().collect();
        self.clauses.push(lits.clone());
        self.sat.add_clause(lits.clone());

        // caddicalは1変数のclauseをclauseだと認めずカウントしてくれないようだ！
        // assert_eq!(self.sat.num_clauses(), self.clauses.len());
    }

    pub fn amo_sequential(&mut self, xs: &[i32]) {
        let k = xs.len();
        if k <= 1 {
            return;
        }
        let mut s = Vec::with_capacity(k - 1);
        for _ in 0..(k - 1) {
            s.push(self.id.next());
        }
        self.clause([-xs[0], s[0]]);
        for i in 1..k - 1 {
            self.clause([-xs[i], s[i]]);
        }
        for i in 1..k {
            self.clause([-xs[i], -s[i - 1]]);
        }
        for i in 1..k - 1 {
            self.clause([-s[i - 1], s[i]]);
        }
    }

    #[inline]
    pub fn choose_one(&mut self, xs: &[i32]) {
        self.clause(xs.iter().copied());
        if xs.len() <= AMO_PAIRWISE_THRESHOLD {
            amo_pairwise(self, xs);
        } else {
            self.amo_sequential(xs);
        }
    }

    pub fn write_dimacs(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        writeln!(f, "p cnf {} {}", self.id.cnt, self.clauses.len())?;
        for c in &self.clauses {
            for &l in c {
                write!(f, "{} ", l)?;
            }
            writeln!(f, "0")?;
        }
        Ok(())
    }
}

// -------------------------- Combinatorial helpers ------------------------

#[inline]
fn compute_diff(door: &[Option<usize>], labels: &[usize]) -> Vec<Vec<bool>> {
    let m = labels.len();
    let mut diff = mat![false; m; m];
    // DP: two positions i,j are distinguishable if label differs OR if next states with same door distinguishable
    for i in (0..m).rev() {
        for j in (0..m).rev() {
            if labels[i] != labels[j] {
                diff[i][j] = true;
            } else if i + 1 < m && j + 1 < m {
                match (door[i], door[j]) {
                    (Some(e1), Some(e2)) if e1 == e2 => {
                        if diff[i + 1][j + 1] {
                            diff[i][j] = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    for i in 0..m {
        diff[i][i] = false;
        for j in 0..i {
            let v = diff[i][j] || diff[j][i];
            diff[i][j] = v;
            diff[j][i] = v;
        }
    }
    diff
}

// ------------------------------ Problem view -----------------------------

struct PlanInfo {
    n: usize,
    labels: Vec<usize>,
    door: Vec<Option<usize>>, // door[i] is the edge used from time i to i+1, or None at plan boundaries/last
    m: usize,
    diff: Vec<Vec<bool>>,
    // Indices in the flattened timeline that correspond to the start of each plan
    starts: Vec<usize>,
}

fn build_info(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> PlanInfo {
    assert_eq!(plans.len(), labels.len());
    let n = num_rooms;

    // Flatten labels and doors with boundary markers (None)
    let mut labels_flat = Vec::new();
    let mut door_flat = Vec::new();
    let mut starts = Vec::with_capacity(plans.len());
    for (p, l) in plans.iter().zip(labels.iter()) {
        assert_eq!(l.len(), p.len() + 1);
        // Append labels and doors for this plan
        // record start index of this plan in the flattened timeline
        starts.push(labels_flat.len());
        // concatenate labels directly; boundaries handled by door=None
        labels_flat.extend_from_slice(l);
        // Doors: for each step in plan push Some(door), and one trailing None for boundary
        for &e in p {
            door_flat.push(Some(e));
        }
        door_flat.push(None); // boundary after this plan
    }
    let m = labels_flat.len();
    assert_eq!(door_flat.len(), m); // last entry must be None for the last plan as well
    let diff = compute_diff(&door_flat, &labels_flat);

    PlanInfo {
        n,
        labels: labels_flat,
        door: door_flat,
        m,
        diff,
        starts,
    }
}

struct Buckets {
    rooms_by_label: [Vec<usize>; 4],
    times_by_label: [Vec<usize>; 4],
}
fn build_buckets(info: &PlanInfo) -> Buckets {
    let mut rooms_by_label: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for u in 0..info.n {
        rooms_by_label[u % 4].push(u);
    }
    let mut times_by_label: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for i in 0..info.m {
        times_by_label[info.labels[i]].push(i);
    }
    Buckets {
        rooms_by_label,
        times_by_label,
    }
}

struct Candidates {
    // V_map[i][u] = Some(var) if room u allowed at time i (label match).
    V_map: Vec<Vec<Option<i32>>>,
}
fn build_candidates(cnf: &mut Cnf, info: &PlanInfo, buckets: &Buckets) -> Candidates {
    let mut V_map = vec![vec![None; info.n]; info.m];
    let mut V_rows: Vec<Vec<i32>> = vec![Vec::new(); info.m];
    for i in 0..info.m {
        let k = info.labels[i];
        let rooms = &buckets.rooms_by_label[k];
        V_rows[i].reserve(rooms.len());
        for &u in rooms {
            let v = cnf.var();
            V_map[i][u] = Some(v);
            V_rows[i].push(v);
        }
        cnf.choose_one(&V_rows[i]);
    }
    Candidates { V_map }
}

// -------------------------- Symmetry breaking ----------------------------

fn first_use_sbp_rect_truncated(cnf: &mut Cnf, W_full: &Vec<Vec<i32>>) {
    let t_all = W_full.len();
    if t_all == 0 {
        return;
    }
    let m = W_full[0].len();
    if m == 0 {
        return;
    }
    let t = std::cmp::min(t_all, m + 2);
    let W = &W_full[0..t];

    let mut z = vec![vec![0i32; m]; t];
    let mut p = vec![vec![0i32; m]; t];
    for i in 0..t {
        for u in 0..m {
            z[i][u] = cnf.var();
            p[i][u] = cnf.var();
        }
    }
    for i in 0..t {
        for u in 0..m {
            cnf.clause([-W[i][u], p[i][u]]);
            cnf.clause([-z[i][u], W[i][u]]);
            cnf.clause([-z[i][u], p[i][u]]);
            if i == 0 {
                cnf.clause([-p[0][u], z[0][u]]);
                cnf.clause([-z[0][u], p[0][u]]);
            } else {
                cnf.clause([-p[i - 1][u], p[i][u]]);
                cnf.clause([-p[i][u], p[i - 1][u], z[i][u]]);
                cnf.clause([-z[i][u], -p[i - 1][u]]);
            }
        }
    }
    for i in 0..t {
        for u in 1..m {
            cnf.clause([-p[i][u], p[i][u - 1]]);
        }
    }
}

fn add_sbp(cnf: &mut Cnf, info: &PlanInfo, buckets: &Buckets, cand: &Candidates) {
    // Per-label rectangular first-use SBP with truncation and anchor earliest to smallest room.
    for k in 0..4 {
        let times = &buckets.times_by_label[k];
        if times.is_empty() {
            continue;
        }
        let rooms = &buckets.rooms_by_label[k];
        let mut W: Vec<Vec<i32>> = Vec::with_capacity(times.len());
        for &i in times {
            let mut row = Vec::with_capacity(rooms.len());
            for &u in rooms {
                let var = cand.V_map[i][u].unwrap();
                row.push(var);
            }
            W.push(row);
        }
        // Anchor earliest occurrence to smallest room of this label
        cnf.clause([W[0][0]]);
        first_use_sbp_rect_truncated(cnf, &W);
    }

    // Also pin the first seen time of each label to the canonical u=k if present.
    let mut seen = [false; 4];
    for i in 0..info.m {
        let k = info.labels[i];
        if !seen[k] {
            seen[k] = true;
            if let Some(v) = cand.V_map[i][k] {
                cnf.clause([v]);
            }
        }
    }
}

fn add_diff_pruning(cnf: &mut Cnf, info: &PlanInfo, buckets: &Buckets, cand: &Candidates) {
    for i in 0..info.m {
        for j in (i + 1)..info.m {
            if info.labels[i] == info.labels[j] && info.diff[i][j] {
                let k = info.labels[i];
                for &u in &buckets.rooms_by_label[k] {
                    let vi = cand.V_map[i][u].unwrap();
                    let vj = cand.V_map[j][u].unwrap();
                    cnf.clause([-vi, -vj]);
                }
            }
        }
    }
}

// Equalization: for pairs (i,j) with same (label,door,next-label) and not yet distinguishable on next,
// enforce (V[i]=u ∧ V[j]=u ∧ V[i+1]=v) -> V[j+1]=v for all u,v in the respective label buckets.
fn add_same_door_equalization(
    cnf: &mut Cnf,
    info: &PlanInfo,
    buckets: &Buckets,
    cand: &Candidates,
) {
    let mut idx_by_ke: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); 6]; 4];
    for i in 0..info.m.saturating_sub(1) {
        if let Some(e) = info.door[i] {
            idx_by_ke[info.labels[i]][e].push(i);
        }
    }
    for k in 0..4 {
        for e in 0..6 {
            let idxs = &idx_by_ke[k][e];
            if idxs.len() <= 1 {
                continue;
            }
            for a in 0..idxs.len() {
                for b in (a + 1)..idxs.len() {
                    let i = idxs[a];
                    let j = idxs[b];
                    if i + 1 >= info.m || j + 1 >= info.m {
                        continue;
                    }
                    if info.labels[i + 1] != info.labels[j + 1] || info.diff[i + 1][j + 1] {
                        continue;
                    }
                    let h = info.labels[i + 1];
                    for &u in &buckets.rooms_by_label[k] {
                        let vi = cand.V_map[i][u].unwrap();
                        let vj = cand.V_map[j][u].unwrap();
                        for &v in &buckets.rooms_by_label[h] {
                            let qi = cand.V_map[i + 1][v].unwrap();
                            let qj = cand.V_map[j + 1][v].unwrap();
                            cnf.clause([-vi, -vj, -qi, qj]);
                        }
                    }
                }
            }
        }
    }
}

// -------------------------- Edge variable layer --------------------------

struct EdgeVars {
    // Tlab[u][e][k]
    Tlab: Vec<Vec<[i32; 4]>>,
    // F[u][e][v]
    F: Vec<Vec<Vec<i32>>>,
    // M[u][v][e][f] symmetric shared
    M: Vec<Vec<[[i32; 6]; 6]>>,
}
fn build_edge_vars(cnf: &mut Cnf, info: &PlanInfo) -> EdgeVars {
    let n = info.n;
    let mut Tlab = vec![vec![[0i32; 4]; 6]; n];
    let mut F = mat![0; n; 6; n];

    for u in 0..n {
        for e in 0..6 {
            let mut trow = [0i32; 4];
            for k in 0..4 {
                Tlab[u][e][k] = cnf.var();
                trow[k] = Tlab[u][e][k];
            }
            cnf.choose_one(&trow);

            let mut frow = Vec::with_capacity(n);
            for v in 0..n {
                F[u][e][v] = cnf.var();
                frow.push(F[u][e][v]);
                cnf.clause([-F[u][e][v], Tlab[u][e][v % 4]]);
            }
            cnf.choose_one(&frow);
        }
    }

    for u in 0..n {
        for e in 0..6 {
            for k in 0..4 {
                cnf.buf.clear();
                cnf.buf.push(-Tlab[u][e][k]);
                for v in (k..n).step_by(4) {
                    cnf.buf.push(F[u][e][v]);
                }
                cnf.clause(cnf.buf.clone());
            }
        }
    }

    let mut M = vec![vec![[[0i32; 6]; 6]; n]; n];
    for u in 0..n {
        for v in u..n {
            for e in 0..6 {
                for f in 0..6 {
                    let var = cnf.var();
                    M[u][v][e][f] = var;
                    M[v][u][f][e] = var;
                }
            }
        }
    }
    // M -> F both endpoints
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                for f in 0..6 {
                    let mv = M[u][v][e][f];
                    cnf.clause([-mv, F[u][e][v]]);
                    cnf.clause([-mv, F[v][f][u]]);
                }
            }
        }
    }
    // Row-wise: F[u][e][v] -> OR_f M[u][v][e][f]; AMO on f
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                let mut row = [0i32; 6];
                for f in 0..6 {
                    row[f] = M[u][v][e][f];
                }
                cnf.buf.clear();
                cnf.buf.push(-F[u][e][v]);
                cnf.buf.extend_from_slice(&row);
                cnf.clause(cnf.buf.clone());
                amo_pairwise(cnf, &row);
            }
        }
    }
    // Column-wise: F[v][f][u] -> OR_e M[u][v][e][f]; AMO on e
    for u in 0..n {
        for v in 0..n {
            for f in 0..6 {
                let mut col = [0i32; 6];
                for e in 0..6 {
                    col[e] = M[u][v][e][f];
                }
                cnf.buf.clear();
                cnf.buf.push(-F[v][f][u]);
                cnf.buf.extend_from_slice(&col);
                cnf.clause(cnf.buf.clone());
                amo_pairwise(cnf, &col);
            }
        }
    }

    EdgeVars { Tlab, F, M }
}

fn add_plan_constraints(
    cnf: &mut Cnf,
    info: &PlanInfo,
    buckets: &Buckets,
    cand: &Candidates,
    edges: &EdgeVars,
) {
    // V[i]=u -> Tlab[u, door[i], labels[i+1]] for valid steps
    for i in 0..info.m.saturating_sub(1) {
        if let Some(e) = info.door[i] {
            let h = info.labels[i + 1];
            let k = info.labels[i];
            for &u in &buckets.rooms_by_label[k] {
                let vi = cand.V_map[i][u].unwrap();
                cnf.clause([-vi, edges.Tlab[u][e][h]]);
            }
        }
    }
    // (V[i]=u ∧ V[i+1]=v) -> F[u, door[i], v]
    for i in 0..info.m.saturating_sub(1) {
        if let Some(e) = info.door[i] {
            let k = info.labels[i];
            let h = info.labels[i + 1];
            for &u in &buckets.rooms_by_label[k] {
                let vi = cand.V_map[i][u].unwrap();
                for &v in &buckets.rooms_by_label[h] {
                    let vj = cand.V_map[i + 1][v].unwrap();
                    cnf.clause([-vi, -vj, edges.F[u][e][v]]);
                }
            }
        }
    }
}

// All plans start from the same room. For each label k that appears at plan starts,
// unify the selected room variable across all start times with that label.
fn add_start_room_unification(
    cnf: &mut Cnf,
    info: &PlanInfo,
    buckets: &Buckets,
    cand: &Candidates,
) {
    // Group start indices by their observed label
    let mut starts_by_label: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for &i in &info.starts {
        let k = info.labels[i];
        starts_by_label[k].push(i);
    }
    for k in 0..4 {
        let starts = &starts_by_label[k];
        if starts.len() <= 1 {
            continue;
        }
        let s0 = starts[0];
        for &si in &starts[1..] {
            for &u in &buckets.rooms_by_label[k] {
                // Enforce equivalence: V[s0,u] <-> V[si,u]
                let v0 = cand.V_map[s0][u].unwrap();
                let vi = cand.V_map[si][u].unwrap();
                cnf.clause([-v0, vi]);
                cnf.clause([-vi, v0]);
            }
        }
    }
}

// -------------------------- Extraction -----------------------------------

fn extract_guess(
    cnf: &Cnf,
    info: &PlanInfo,
    buckets: &Buckets,
    cand: &Candidates,
    edges: &EdgeVars,
) -> Guess {
    let n = info.n;
    let mut guess = Guess {
        start: 0,
        rooms: vec![0; n],
        graph: vec![[(!0, !0); 6]; n],
    };

    // Start room: find true variable at time 0
    {
        let k0 = info.labels[0];
        let rooms0 = &buckets.rooms_by_label[k0];
        let mut s = rooms0[0];
        for &u in rooms0 {
            let v = cand.V_map[0][u].unwrap();
            if cnf.sat.value(v) == Some(true) {
                s = u;
                break;
            }
        }
        guess.start = s;
    }

    for u in 0..n {
        guess.rooms[u] = u % 4;
    }

    for u in 0..n {
        for e in 0..6 {
            let mut v_sel = 0usize;
            for v in 0..n {
                if cnf.sat.value(edges.F[u][e][v]) == Some(true) {
                    v_sel = v;
                    break;
                }
            }
            let mut f_sel = 0usize;
            for f in 0..6 {
                if cnf.sat.value(edges.M[u][v_sel][e][f]) == Some(true) {
                    f_sel = f;
                    break;
                }
            }
            guess.graph[u][e] = (v_sel, f_sel);
        }
    }
    guess
}

// -------------------------- CNF construction wrapper ---------------------

fn build_cnf_for_plans(
    num_rooms: usize,
    plans: &Vec<Vec<usize>>,
    labels: &Vec<Vec<usize>>,
) -> (PlanInfo, Buckets, Cnf, Candidates, EdgeVars) {
    // 1) Build flattened info from provided plans and labels
    let info = build_info(num_rooms, plans, labels);

    // 2) Build buckets and candidates
    let buckets = build_buckets(&info);
    let mut cnf = Cnf::new();
    let cand = build_candidates(&mut cnf, &info, &buckets);

    // 3) Add pruning and symmetry breaking
    add_diff_pruning(&mut cnf, &info, &buckets, &cand);
    add_sbp(&mut cnf, &info, &buckets, &cand);
    add_same_door_equalization(&mut cnf, &info, &buckets, &cand);

    // 4) Edge layer and plan constraints
    let edges = build_edge_vars(&mut cnf, &info);
    add_plan_constraints(&mut cnf, &info, &buckets, &cand, &edges);
    // 4.5) Unify starting room across all plans
    add_start_room_unification(&mut cnf, &info, &buckets, &cand);

    (info, buckets, cnf, cand, edges)
}

pub fn solve(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> Guess {
    let (info, buckets, mut cnf, cand, edges) = build_cnf_for_plans(num_rooms, plans, labels);

    // 5) Solve
    assert_eq!(cnf.sat.solve(), Some(true));
    let guess = extract_guess(&cnf, &info, &buckets, &cand, &edges);
    assert!(check_explore(&guess, plans, labels));
    guess
}

/// Fixes a prefix of edges in the graph irrespective of specific times.
/// Each tuple is `(u, e, v, f_opt)` meaning force `F[u][e][v]` and optionally `M[u][v][e][f]`.
/// Returns `None` if the resulting CNF is unsatisfiable.
pub fn solve_with_edge_prefix_fixed(
    num_rooms: usize,
    plans: &Vec<Vec<usize>>,
    labels: &Vec<Vec<usize>>,
    prefix: &[(usize, usize, usize, Option<usize>)],
) -> Option<Guess> {
    // 1) Build flattened info from provided plans and labels
    let info = build_info(num_rooms, plans, labels);

    // 2) Build buckets and candidates
    let buckets = build_buckets(&info);
    let mut cnf = Cnf::new();
    let cand = build_candidates(&mut cnf, &info, &buckets);

    // 3) Add pruning and symmetry breaking
    add_diff_pruning(&mut cnf, &info, &buckets, &cand);
    add_sbp(&mut cnf, &info, &buckets, &cand);
    add_same_door_equalization(&mut cnf, &info, &buckets, &cand);

    // 4) Edge layer and plan constraints
    let edges = build_edge_vars(&mut cnf, &info);
    // Apply prefix edge constraints
    for &(u, e, v, f_opt) in prefix.iter() {
        if u >= info.n || v >= info.n || e >= 6 {
            return None;
        }
        cnf.clause([edges.F[u][e][v]]);
        if let Some(f) = f_opt {
            if f >= 6 {
                return None;
            }
            cnf.clause([edges.M[u][v][e][f]]);
        }
    }
    add_plan_constraints(&mut cnf, &info, &buckets, &cand, &edges);
    add_start_room_unification(&mut cnf, &info, &buckets, &cand);

    // 5) Solve
    match cnf.sat.solve() {
        Some(true) => {
            let guess = extract_guess(&cnf, &info, &buckets, &cand, &edges);
            assert!(check_explore(&guess, plans, labels));
            Some(guess)
        }
        _ => None,
    }
}

// ------------------------------ Portfolio Solver -------------------------------------

pub struct SATSolver {
    pub path: String,
    pub args: Vec<String>,
}

pub fn launch_portfolio(
    dimacs_path: &std::path::Path,
    solvers: &[SATSolver],
) -> std::collections::HashSet<i32> {
    use std::collections::HashSet;
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;

    assert!(!solvers.is_empty(), "no solvers provided");

    // Spawn all solvers
    let mut children: Vec<Arc<Mutex<Child>>> = Vec::with_capacity(solvers.len());
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(solvers.len());

    for (idx, s) in solvers.iter().enumerate() {
        let mut child = Command::new(&s.path)
            .args(&s.args)
            .arg(dimacs_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("failed to spawn portfolio solver");

        let stdout = child
            .stdout
            .take()
            .expect("failed to capture solver stdout");
        let child = Arc::new(Mutex::new(child));
        children.push(Arc::clone(&child));

        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            let mut saw_v = false;
            let mut saw_unsat = false;
            let mut buf = String::new();

            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                // Mirror child stdout to our stdout for real-time progress.
                // println!("{}", line);
                let _ = std::io::stdout().flush();
                if line.starts_with('s') || line.starts_with('S') {
                    if line.to_ascii_lowercase().contains("unsat") {
                        saw_unsat = true;
                    }
                } else if line.starts_with('v') || line.starts_with('V') {
                    saw_v = true;
                    buf.push_str(&line);
                    buf.push('\n');
                }
            }

            // Wait for exit after stdout closed
            let status = child.lock().unwrap().wait();
            let code = status.ok().and_then(|s| s.code());
            let _ = tx.send((idx, code, buf, saw_unsat, saw_v));
        }));
    }

    drop(tx); // close sender in main thread

    // Receive first acceptable result
    let mut winner: Option<(usize, String)> = None;
    for received in rx.iter() {
        let (idx, code, buf, saw_unsat, saw_v) = received;
        if (code == Some(0) || code == Some(10)) && !saw_unsat && saw_v {
            // Announce winner solver
            let s = &solvers[idx];
            eprintln!("Portfolio winner: {} {}", s.path, s.args.join(" "));
            winner = Some((idx, buf));
            break;
        }
    }

    // Kill all losers
    if let Some((win_idx, _)) = &winner {
        for (i, ch) in children.iter().enumerate() {
            if i != *win_idx {
                let _ = ch.lock().unwrap().kill();
            }
        }
    } else {
        // No winner found; ensure all are terminated
        for ch in &children {
            let _ = ch.lock().unwrap().kill();
        }
    }

    // Join all threads to complete cleanup
    for h in handles {
        let _ = h.join();
    }

    let (_, buf) = winner.expect("no solver produced a satisfiable model");

    // Parse 'v' lines into a model set
    let mut solution: HashSet<i32> = HashSet::new();
    for line in buf.lines() {
        if !(line.starts_with('v') || line.starts_with('V')) {
            continue;
        }
        for tok in line.split_whitespace() {
            if tok == "v" || tok == "V" {
                continue;
            }
            if let Ok(x) = tok.parse::<i32>() {
                if x == 0 {
                    break;
                }
                solution.insert(x);
            }
        }
    }
    assert!(
        !solution.is_empty(),
        "winner solver produced no 'v' assignment lines"
    );
    solution
}

// High-level: build CNF, write DIMACS, run portfolio, inject model, extract Guess
pub fn solve_portfolio(
    num_rooms: usize,
    plans: &Vec<Vec<usize>>,
    labels: &Vec<Vec<usize>>,
    solvers: &[SATSolver],
    dimacs_path: &std::path::Path,
) -> Guess {
    // 1) CNF 構築（solve と共通化）
    let (info, buckets, mut cnf, cand, edges) = build_cnf_for_plans(num_rooms, plans, labels);

    // 2) DIMACS 書き出し
    cnf.write_dimacs(dimacs_path)
        .expect("failed to write DIMACS");
    eprintln!(
        "Original: num_clauses={}, num_variables={}, clauses={}",
        cnf.sat.num_clauses(),
        cnf.sat.num_variables(),
        cnf.clauses.len(),
    );

    // 3) 外部ソルバを並列実行（ポートフォリオ）
    let solution = launch_portfolio(dimacs_path, solvers);

    // 4) モデルを単位節として注入 → CaDiCaL で充足化
    for &v in &solution {
        cnf.clause([v]);
    }
    assert_eq!(cnf.sat.solve(), Some(true));
    for &v in &solution {
        assert_eq!(cnf.sat.value(v.abs()), Some(v > 0));
    }

    // 5) 既存の抽出ロジックをそのまま利用
    let guess = extract_guess(&cnf, &info, &buckets, &cand, &edges);
    assert!(check_explore(&guess, plans, labels));
    guess
}

pub fn solve_cadical_multi(
    num_rooms: usize,
    plans: &Vec<Vec<usize>>,
    labels: &Vec<Vec<usize>>,
    n_workers: usize,
) -> Guess {
    let cadical_path = std::env::var("CADICAL_PATH")
        .unwrap_or_else(|_| "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned());

    let solvers = (0..n_workers)
        .map(|seed| SATSolver {
            path: cadical_path.to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        })
        .collect_vec();

    let dimacs_path = format!("tmp/{}.cnf", std::process::id());
    let dimacs_path = Path::new(&dimacs_path);
    if let Some(parent) = dimacs_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    solve_portfolio(num_rooms, &plans, &labels, &solvers, dimacs_path)
}

pub fn solve_cnf_parallel(cnf: &mut Cnf, n_cadical_workers: usize, n_kissat_workers: usize) {
    let cadical_path = std::env::var("CADICAL_PATH")
        .unwrap_or_else(|_| "/home/iwiwi/tmp/cadical-rel-2.1.3/build/cadical".to_owned());

    let kissat_path = std::env::var("KISSAT_PATH")
        .unwrap_or_else(|_| "/home/iwiwi/tmp/kissat-4.0.3-linux-amd64".to_owned());

    let solvers: Vec<SATSolver> = (0..n_cadical_workers)
        .map(|seed| SATSolver {
            path: cadical_path.to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        })
        .chain((0..n_kissat_workers).map(|seed| SATSolver {
            path: kissat_path.to_owned(),
            args: [format!("--seed={}", seed), "--sat".to_owned()].to_vec(),
        }))
        .collect_vec();

    let dimacs_path = format!("tmp/{}.cnf", std::process::id());
    let dimacs_path = Path::new(&dimacs_path);
    if let Some(parent) = dimacs_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    cnf.write_dimacs(dimacs_path).unwrap();
    let solution = launch_portfolio(dimacs_path, &solvers);

    for &v in &solution {
        cnf.clause([v]);
    }
    assert_eq!(cnf.sat.solve(), Some(true));
}
