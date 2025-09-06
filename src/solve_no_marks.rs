#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]

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

pub fn amo_pairwise(sat: &mut cadical::Solver, xs: &[i32]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            sat.add_clause([-xs[i], -xs[j]]);
        }
    }
}

pub fn amo_sequential(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    let k = xs.len();
    if k <= 1 {
        return;
    }
    let mut s = Vec::with_capacity(k - 1);
    for _ in 0..(k - 1) {
        s.push(id.next());
    }
    sat.add_clause([-xs[0], s[0]]);
    for i in 1..k - 1 {
        sat.add_clause([-xs[i], s[i]]);
    }
    for i in 1..k {
        sat.add_clause([-xs[i], -s[i - 1]]);
    }
    for i in 1..k - 1 {
        sat.add_clause([-s[i - 1], s[i]]);
    }
}

pub fn choose_one(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    sat.add_clause(xs.iter().copied());
    if xs.len() <= AMO_PAIRWISE_THRESHOLD {
        amo_pairwise(sat, xs);
    } else {
        amo_sequential(sat, xs, id);
    }
}

pub struct Cnf {
    pub sat: cadical::Solver,
    id: Counter,
    buf: Vec<i32>,
}

impl Cnf {
    pub fn new() -> Self {
        Self {
            sat: cadical::Solver::with_config("sat").unwrap(),
            id: Counter::new(),
            buf: Vec::with_capacity(128),
        }
    }
    #[inline]
    pub fn var(&mut self) -> i32 {
        self.id.next()
    }
    #[inline]
    pub fn clause<I: IntoIterator<Item = i32>>(&mut self, lits: I) {
        self.sat.add_clause(lits);
    }
    #[inline]
    pub fn choose_one(&mut self, xs: &[i32]) {
        choose_one(&mut self.sat, xs, &mut self.id);
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
}

fn build_info(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> PlanInfo {
    assert_eq!(plans.len(), labels.len());
    let n = num_rooms;

    // Flatten labels and doors with boundary markers (None)
    let mut labels_flat = Vec::new();
    let mut door_flat = Vec::new();
    for (p, l) in plans.iter().zip(labels.iter()) {
        assert_eq!(l.len(), p.len() + 1);
        // Append labels and doors for this plan
        if labels_flat.is_empty() {
            // first plan: push all labels
            labels_flat.extend_from_slice(l);
        } else {
            // subsequent plans: concatenate labels directly; boundaries handled by door=None
            labels_flat.extend_from_slice(l);
        }
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
                amo_pairwise(&mut cnf.sat, &row);
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
                amo_pairwise(&mut cnf.sat, &col);
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

// ------------------------------ Public solve ------------------------------

pub fn solve(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> Guess {
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

    // 5) Solve
    assert_eq!(cnf.sat.solve(), Some(true));

    // 6) Extract and verify
    let guess = extract_guess(&cnf, &info, &buckets, &cand, &edges);
    assert!(check_explore(&guess, plans, labels));
    guess
}
