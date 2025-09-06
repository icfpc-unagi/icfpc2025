// EVOLVE-BLOCK-START
#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::{judge::*, *};
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;
use std::env;

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

struct PlanInfo {
    n: usize,
    plan: Vec<usize>,
    labels: Vec<usize>,
    t: usize,
    m: usize,
    diff: Vec<Vec<bool>>,
}
fn balanced_plan(n: usize, rng: &mut ChaCha12Rng) -> Vec<usize> {
    let len = 18 * n;
    let mut plan = Vec::with_capacity(len);
    for d in 0..6 {
        for _ in 0..(len / 6) {
            plan.push(d);
        }
    }
    plan.shuffle(rng);
    plan
}

/*
fn acquire_plan_and_labels(judge: &mut dyn icfpc2025::judge::Judge) -> PlanInfo {
    let n = judge.num_rooms();
    let mut rng = ChaCha12Rng::seed_from_u64(0xC0FF_EE42);
    let plan = balanced_plan(n, &mut rng);
    let steps: Vec<(Option<usize>, usize)> = plan.iter().copied().map(|d| (None, d)).collect();
    let labels = judge.explore(&[steps])[0].clone();
    let m = labels.len();
    let t = plan.len();
    debug_assert_eq!(m, t + 1);
    let diff = compute_diff(&plan, &labels);
    PlanInfo { n, plan, labels, t, m, diff }
}
*/

fn acquire_plan_and_labels(judge: &mut dyn icfpc2025::judge::Judge) -> PlanInfo {
    let n = judge.num_rooms();

    // Default plan strings for supported sizes
    let default_plan_str = if n == 30 {
        "413022551403315200442351124530532105441250342013450431221500235401332245541100524301142035531432234405215311350240015425104334025123304521120534103522445503114230032542135431452051002415012340523321554433021451132041025533014400351220440115324321342250451305502315412031045522433500142534115203442231104350310540221435132441500553020145133425523012412033443004231254411023052214500135410325345320121545042102113352405514315340154304422003355523411201445331322002514054225105033004323315550112134421441352532400511024124025405311304432221235"
    } else if n == 24 {
        "053421124355003145223044132102540153351203445023114200554324125133051042215014033152443520411325530244002234511032054154230134552103501221433402532514310044152332500144551240530123153410521354220330420524115043021334514011522400543355322502431104320154423513402104531230554420011342541350314220511225053310324405552341300214450322545125330150043123141012421453202513005434045013322443102352331551412002403415510035111204255404452032"
    } else {
        panic!("Unsupported number of rooms: {}", n);
    };

    // Allow override via environment variable PLAN_STR (string of digits 0-5)
    let plan_str = env::var("PLAN_STR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_plan_str.to_string());

    let plan = plan_str
        .chars()
        .map(|c| c.to_digit(10).unwrap() as usize)
        .collect::<Vec<_>>();

    let steps: Vec<(Option<usize>, usize)> = plan.iter().copied().map(|d| (None, d)).collect();
    let labels = judge.explore(&[steps])[0].clone();
    let m = labels.len();
    let t = plan.len();
    debug_assert_eq!(m, t + 1);
    let diff = compute_diff(&plan, &labels);
    PlanInfo {
        n,
        plan,
        labels,
        t,
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
    // V_rows[i] = list of variables for time i.
    V_rows: Vec<Vec<i32>>,
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
    for i in 0..info.t {
        idx_by_ke[info.labels[i]][info.plan[i]].push(i);
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
    // V[i]=u -> Tlab[u, plan[i], labels[i+1]]
    for i in 0..info.t {
        let e = info.plan[i];
        let h = info.labels[i + 1];
        let k = info.labels[i];
        for &u in &buckets.rooms_by_label[k] {
            let vi = cand.V_map[i][u].unwrap();
            cnf.clause([-vi, edges.Tlab[u][e][h]]);
        }
    }
    // (V[i]=u ∧ V[i+1]=v) -> F[u, plan[i], v]
    for i in 0..info.t {
        let e = info.plan[i];
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

// ------------------------------ Main solve -------------------------------

fn solve(judge: &mut dyn icfpc2025::judge::Judge) -> bool {
    // 1) Acquire single explore trace with a balanced, shuffled plan
    let info = acquire_plan_and_labels(judge);
    let mut diff_count = 0;
    for i in 0..info.labels.len() {
        for j in 0..i {
            if info.diff[i][j] {
                diff_count += 1;
            }
        }
    }
    eprintln!("diff_count = {}", diff_count);
    // if diff_count < 113800 {
    //     return false;
    // }
    let mut aib = mat![false; 4; 6; 4];
    for k in 0..info.plan.len() {
        let a = info.labels[k];
        let i = info.plan[k];
        let b = info.labels[k + 1];
        aib[a][i][b] = true;
    }
    let mut cnt = 0;
    for a in 0..4 {
        for i in 0..6 {
            for b in 0..4 {
                if !aib[a][i][b] {
                    cnt += 1;
                }
            }
        }
    }
    eprintln!("aib_missing = {}", cnt);
    let mut label_door = mat![0; 4; 6];
    for i in 0..info.plan.len() {
        let door = info.plan[i];
        let label = info.labels[i];
        label_door[label][door] += 1;
    }
    let mut sum = 0.0;
    let mut num = vec![0; 4];
    for i in 0..info.n {
        num[i % 4] += 1;
    }
    for i in 0..4 {
        for j in 0..6 {
            let expected = num[i] as f64 / info.n as f64 * info.plan.len() as f64 / 6.0;
            sum += (expected - label_door[i][j] as f64).powi(2);
        }
    }
    eprintln!("label-door-chi2 = {}", sum);
    if sum > 200.0 {
        return false;
    }
    eprintln!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

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
    assert!(check_explore(
        &guess,
        &[info.plan.clone()],
        &[info.labels.clone()]
    ));
    judge.guess(&guess)
}
// EVOLVE-BLOCK-END

fn main() {
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    while !solve(judge.as_mut()) {
        judge.restart();
    }
}
