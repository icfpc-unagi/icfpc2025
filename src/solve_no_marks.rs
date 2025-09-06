#![allow(
    non_snake_case,
    dead_code,
    clippy::if_same_then_else,
    clippy::ptr_arg,
    clippy::manual_memcpy,
    clippy::needless_range_loop
)]
use crate::{judge::Guess, mat};

// ----------------------------- CNF utilities -----------------------------

struct Counter {
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

#[inline]
fn amo_pairwise(sat: &mut cadical::Solver, xs: &[i32]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            sat.add_clause([-xs[i], -xs[j]]);
        }
    }
}

#[inline]
fn amo_sequential(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
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

#[inline]
fn choose_one(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    sat.add_clause(xs.iter().copied());
    if xs.len() <= AMO_PAIRWISE_THRESHOLD {
        amo_pairwise(sat, xs);
    } else {
        amo_sequential(sat, xs, id);
    }
}

struct Cnf {
    sat: cadical::Solver,
    id: Counter,
    buf: Vec<i32>,
}
impl Cnf {
    fn new() -> Self {
        Self {
            sat: cadical::Solver::with_config("sat").unwrap(),
            id: Counter::new(),
            buf: Vec::with_capacity(128),
        }
    }
    #[inline]
    fn var(&mut self) -> i32 {
        self.id.next()
    }
    #[inline]
    fn clause<I: IntoIterator<Item = i32>>(&mut self, lits: I) {
        self.sat.add_clause(lits);
    }
    #[inline]
    fn choose_one(&mut self, xs: &[i32]) {
        choose_one(&mut self.sat, xs, &mut self.id);
    }
}

// -------------------------- Combinatorial helpers ------------------------

#[inline]
fn compute_diff(plan: &[usize], labels: &[usize]) -> Vec<Vec<bool>> {
    let m = labels.len();
    let t = plan.len();
    let mut diff = mat![false; m; m];
    for i in (0..m).rev() {
        for j in (0..m).rev() {
            if labels[i] != labels[j] {
                diff[i][j] = true;
            } else if i < t && j < t && plan[i] == plan[j] && diff[i + 1][j + 1] {
                diff[i][j] = true;
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

struct MultiInfo {
    n: usize,
    plans: Vec<Vec<usize>>,  // per plan
    labels: Vec<Vec<usize>>, // per plan
    ts: Vec<usize>,          // per plan
    ms: Vec<usize>,          // per plan
    offsets: Vec<usize>,     // per plan -> global time offset
    m_total: usize,          // total timepoints across all plans
    labels_global: Vec<usize>, // flattened labels by global time
    diffs: Vec<Vec<Vec<bool>>>, // per plan diff matrix
}

fn build_multi_info(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> MultiInfo {
    assert_eq!(plans.len(), labels.len());
    let q = plans.len();
    let mut ts = Vec::with_capacity(q);
    let mut ms = Vec::with_capacity(q);
    let mut diffs = Vec::with_capacity(q);
    for p in 0..q {
        let t = plans[p].len();
        let m = labels[p].len();
        assert_eq!(m, t + 1);
        ts.push(t);
        ms.push(m);
        diffs.push(compute_diff(&plans[p], &labels[p]));
    }
    let mut offsets = Vec::with_capacity(q);
    let mut sum = 0usize;
    for &m in &ms {
        offsets.push(sum);
        sum += m;
    }
    let m_total = sum;
    let mut labels_global = vec![0usize; m_total];
    for p in 0..q {
        let off = offsets[p];
        for i in 0..ms[p] {
            labels_global[off + i] = labels[p][i];
        }
    }
    MultiInfo {
        n: num_rooms,
        plans: plans.clone(),
        labels: labels.clone(),
        ts,
        ms,
        offsets,
        m_total,
        labels_global,
        diffs,
    }
}

struct Buckets {
    rooms_by_label: [Vec<usize>; 4],
    times_by_label: [Vec<usize>; 4], // global time indices
}

fn build_buckets(info: &MultiInfo) -> Buckets {
    let mut rooms_by_label: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for u in 0..info.n {
        rooms_by_label[u % 4].push(u);
    }
    let mut times_by_label: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for g in 0..info.m_total {
        let k = info.labels_global[g];
        times_by_label[k].push(g);
    }
    Buckets {
        rooms_by_label,
        times_by_label,
    }
}

struct Candidates {
    // V_map[global_time][u] = Some(var) if room u allowed at that time (label match)
    V_map: Vec<Vec<Option<i32>>>,
    // V_rows[global_time] = list of variables for that time
    V_rows: Vec<Vec<i32>>,
}

fn build_candidates(cnf: &mut Cnf, info: &MultiInfo, buckets: &Buckets) -> Candidates {
    let mut V_map = vec![vec![None; info.n]; info.m_total];
    let mut V_rows: Vec<Vec<i32>> = vec![Vec::new(); info.m_total];
    for g in 0..info.m_total {
        let k = info.labels_global[g];
        let rooms = &buckets.rooms_by_label[k];
        V_rows[g].reserve(rooms.len());
        for &u in rooms {
            let v = cnf.var();
            V_map[g][u] = Some(v);
            V_rows[g].push(v);
        }
        cnf.choose_one(&V_rows[g]);
    }
    Candidates { V_map, V_rows }
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

fn add_sbp(cnf: &mut Cnf, info: &MultiInfo, buckets: &Buckets, cand: &Candidates) {
    // Per-label rectangular first-use SBP with truncation and anchor earliest to smallest room.
    for k in 0..4 {
        let times = &buckets.times_by_label[k];
        if times.is_empty() {
            continue;
        }
        let rooms = &buckets.rooms_by_label[k];
        let mut W: Vec<Vec<i32>> = Vec::with_capacity(times.len());
        for &g in times {
            let mut row = Vec::with_capacity(rooms.len());
            for &u in rooms {
                let var = cand.V_map[g][u].unwrap();
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
    for g in 0..info.m_total {
        let k = info.labels_global[g];
        if !seen[k] {
            seen[k] = true;
            if let Some(v) = cand.V_map[g][k] {
                cnf.clause([v]);
            }
        }
    }
}

fn add_diff_pruning(cnf: &mut Cnf, info: &MultiInfo, buckets: &Buckets, cand: &Candidates) {
    // Within each plan, for equal labels but distinguishable times, forbid same room.
    let q = info.plans.len();
    for p in 0..q {
        let off = info.offsets[p];
        for i in 0..info.ms[p] {
            for j in (i + 1)..info.ms[p] {
                if info.labels[p][i] == info.labels[p][j] && info.diffs[p][i][j] {
                    let k = info.labels[p][i];
                    for &u in &buckets.rooms_by_label[k] {
                        let vi = cand.V_map[off + i][u].unwrap();
                        let vj = cand.V_map[off + j][u].unwrap();
                        cnf.clause([-vi, -vj]);
                    }
                }
            }
        }
    }
}

// Equalization: for pairs (i,j) with same (label,door,next-label) and not yet distinguishable on next,
// enforce (V[i]=u ∧ V[j]=u ∧ V[i+1]=v) -> V[j+1]=v for all u,v in the respective label buckets.
fn add_same_door_equalization(
    cnf: &mut Cnf,
    info: &MultiInfo,
    buckets: &Buckets,
    cand: &Candidates,
) {
    let q = info.plans.len();
    for p in 0..q {
        // index by (label, door) for this plan
        let mut idx_by_ke: Vec<Vec<Vec<usize>>> = vec![vec![Vec::new(); 6]; 4];
        for i in 0..info.ts[p] {
            idx_by_ke[info.labels[p][i]][info.plans[p][i]].push(i);
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
                        // Require same next-label and not distinguishable next
                        if info.labels[p][i + 1] != info.labels[p][j + 1]
                            || info.diffs[p][i + 1][j + 1]
                        {
                            continue;
                        }
                        let h = info.labels[p][i + 1];
                        let off = info.offsets[p];
                        for &u in &buckets.rooms_by_label[k] {
                            let vi = cand.V_map[off + i][u].unwrap();
                            let vj = cand.V_map[off + j][u].unwrap();
                            for &v in &buckets.rooms_by_label[h] {
                                let qi = cand.V_map[off + i + 1][v].unwrap();
                                let qj = cand.V_map[off + j + 1][v].unwrap();
                                cnf.clause([-vi, -vj, -qi, qj]);
                            }
                        }
                    }
                }
            }
        }
    }
}

// Enforce that all plans start at the same room as plan 0 (both directions equality per room u).
fn add_start_unification(cnf: &mut Cnf, info: &MultiInfo, buckets: &Buckets, cand: &Candidates) {
    let q = info.plans.len();
    if q <= 1 {
        return;
    }
    let off0 = info.offsets[0];
    let k0 = info.labels[0][0];
    for p in 1..q {
        let offp = info.offsets[p];
        let kp = info.labels[p][0];
        assert_eq!(kp, k0, "All plans must start with the same observed label");
        for &u in &buckets.rooms_by_label[k0] {
            let v0 = cand.V_map[off0][u].unwrap();
            let vp = cand.V_map[offp][u].unwrap();
            // v0 <-> vp
            cnf.clause([-v0, vp]);
            cnf.clause([-vp, v0]);
        }
    }
}

// -------------------------- Edge variable layer --------------------------

struct EdgeVars {
    // Tlab[u][e][h] = door e from room u leads to a room with label h
    Tlab: Vec<Vec<Vec<i32>>>,
    // F[u][e][v] = door e from room u leads to room v
    F: Vec<Vec<Vec<i32>>>,
    // M[u][v][e][f] = edge from (u,e) to (v,f)
    M: Vec<Vec<Vec<Vec<i32>>>>,
}

fn build_edge_vars(cnf: &mut Cnf, info: &MultiInfo) -> EdgeVars {
    let n = info.n;
    let mut Tlab = vec![vec![vec![0i32; 4]; 6]; n];
    for u in 0..n {
        for e in 0..6 {
            for h in 0..4 {
                Tlab[u][e][h] = cnf.var();
            }
            // Exactly one next-label per (u,e)
            cnf.choose_one(&Tlab[u][e]);
        }
    }

    let mut F = vec![vec![vec![0i32; n]; 6]; n];
    for u in 0..n {
        for e in 0..6 {
            for v in 0..n {
                F[u][e][v] = cnf.var();
            }
            // Exactly one target room per (u,e)
            cnf.choose_one(&F[u][e]);
        }
    }

    // For each (u,v), the doors must pair bijectively
    let mut M = vec![vec![vec![vec![0i32; 6]; 6]; n]; n];
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                for f in 0..6 {
                    M[u][v][e][f] = cnf.var();
                }
            }
        }
    }
    // Tie M and F and enforce bijection per (u,v)
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                for f in 0..6 {
                    // M[u][v][e][f] -> F[u][e][v]
                    cnf.clause([-M[u][v][e][f], F[u][e][v]]);
                    // M[u][v][e][f] -> F[v][f][u]
                    cnf.clause([-M[u][v][e][f], F[v][f][u]]);
                    // F[u][e][v] ∧ F[v][f][u] -> M[u][v][e][f]
                    cnf.clause([-F[u][e][v], -F[v][f][u], M[u][v][e][f]]);
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
    info: &MultiInfo,
    buckets: &Buckets,
    cand: &Candidates,
    edges: &EdgeVars,
) {
    // V[g]=u -> Tlab[u, plan[p][i], labels[p][i+1]]
    // (V[g]=u ∧ V[g+1]=v) -> F[u, plan[p][i], v]
    let q = info.plans.len();
    for p in 0..q {
        let off = info.offsets[p];
        for i in 0..info.ts[p] {
            let e = info.plans[p][i];
            let k = info.labels[p][i];
            let h = info.labels[p][i + 1];
            for &u in &buckets.rooms_by_label[k] {
                let vi = cand.V_map[off + i][u].unwrap();
                cnf.clause([-vi, edges.Tlab[u][e][h]]);
                for &v in &buckets.rooms_by_label[h] {
                    let vj = cand.V_map[off + i + 1][v].unwrap();
                    cnf.clause([-vi, -vj, edges.F[u][e][v]]);
                }
            }
        }
    }
}

// -------------------------- Extraction -----------------------------------

fn extract_guess(
    cnf: &Cnf,
    info: &MultiInfo,
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

    // Start room: find true variable at the first global time (plan 0, time 0)
    {
        let g0 = info.offsets[0];
        let k0 = info.labels_global[g0];
        let rooms0 = &buckets.rooms_by_label[k0];
        let mut s = rooms0[0];
        for &u in rooms0 {
            let v = cand.V_map[g0][u].unwrap();
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

// ------------------------------ Main solve -------------------------------

pub fn solve(num_rooms: usize, plans: &Vec<Vec<usize>>, labels: &Vec<Vec<usize>>) -> Guess {
    // 1) Multi-plan info assembly
    let info = build_multi_info(num_rooms, plans, labels);

    // 2) Buckets and candidates
    let buckets = build_buckets(&info);
    let mut cnf = Cnf::new();
    let cand = build_candidates(&mut cnf, &info, &buckets);

    // 3) Pruning and symmetry breaking
    add_start_unification(&mut cnf, &info, &buckets, &cand);
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
    assert!(crate::judge::check_explore(
        &guess,
        plans,
        labels
    ));
    guess
}
