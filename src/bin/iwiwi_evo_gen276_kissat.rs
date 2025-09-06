// EVOLVE-BLOCK-START (ported to rustsat + kissat)
#![allow(
    clippy::needless_range_loop,
    clippy::useless_vec,
    clippy::partialeq_to_none,
    clippy::if_same_then_else,
    clippy::ptr_arg,
    clippy::manual_memcpy,
    clippy::cloned_ref_to_slice_refs,
    clippy::vec_init_then_push,
    clippy::match_like_matches_macro,
    non_snake_case,
    unused_variables,
    dead_code
)]
use icfpc2025::{judge::*, *};
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;

use rustsat::clause;
use rustsat::instances::SatInstance;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Assignment, Lit, TernaryVal};

// ----------------------------- CNF utilities -----------------------------

const AMO_PAIRWISE_THRESHOLD: usize = 6;

#[inline]
fn amo_pairwise(inst: &mut SatInstance, xs: &[Lit]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            inst.add_clause(clause![!xs[i], !xs[j]]);
        }
    }
}

#[inline]
fn amo_sequential(inst: &mut SatInstance, xs: &[Lit]) {
    let k = xs.len();
    if k <= 1 {
        return;
    }
    let mut s: Vec<Lit> = Vec::with_capacity(k - 1);
    for _ in 0..(k - 1) {
        s.push(inst.new_lit());
    }
    inst.add_clause(clause![!xs[0], s[0]]);
    for i in 1..k - 1 {
        inst.add_clause(clause![!xs[i], s[i]]);
    }
    for i in 1..k {
        inst.add_clause(clause![!xs[i], !s[i - 1]]);
    }
    for i in 1..k - 1 {
        inst.add_clause(clause![!s[i - 1], s[i]]);
    }
}

#[inline]
fn choose_one(inst: &mut SatInstance, xs: &[Lit]) {
    // ALO
    {
        let mut c = Vec::with_capacity(xs.len());
        c.extend_from_slice(xs);
        inst.add_clause(c.as_slice().into());
    }
    // AMO
    if xs.len() <= AMO_PAIRWISE_THRESHOLD {
        amo_pairwise(inst, xs);
    } else {
        amo_sequential(inst, xs);
    }
}

struct Cnf {
    inst: SatInstance,
    buf: Vec<Lit>,
}
impl Cnf {
    fn new() -> Self {
        Self {
            inst: SatInstance::new(),
            buf: Vec::with_capacity(128),
        }
    }
    #[inline]
    fn var(&mut self) -> Lit {
        self.inst.new_lit()
    }
    #[inline]
    fn clause_slice(&mut self, lits: &[Lit]) {
        self.inst.add_clause(lits.into());
    }
    #[inline]
    fn add_unit(&mut self, l: Lit) {
        self.inst.add_unit(l);
    }
    #[inline]
    fn choose_one(&mut self, xs: &[Lit]) {
        choose_one(&mut self.inst, xs);
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
    let labels = judge.explore(&vec![plan.clone()])[0].clone();
    let m = labels.len();
    let t = plan.len();
    debug_assert_eq!(m, t + 1);
    let diff = compute_diff(&plan, &labels);
    PlanInfo { n, plan, labels, t, m, diff }
}
*/

fn acquire_plan_and_labels(judge: &mut dyn icfpc2025::judge::Judge) -> PlanInfo {
    let n = judge.num_rooms();

    let plan_str = if n == 30 {
        "413022551403315200442351124530532105441250342013450431221500235401332245541100524301142035531432234405215311350240015425104334025123304521120534103522445503114230032542135431452051002415012340523321554433021451132041025533014400351220440115324321342250451305502315412031045522433500142534115203442231104350310540221435132441500553020145133425523012412033443004231254411023052214500135410325345320121545042102113352405514315340154304422003355523411201445331322002514054225105033004323315550112134421441352532400511024124025405311304432221235"
    } else if n == 24 {
        "053421124355003145223044132102540153351203445023114200554324125133051042215014033152443520411325530244002234511032054154230134552103501221433402532514310044152332500144551240530123153410521354220330420524115043021334514011522400543355322502431104320154423513402104531230554420011342541350314220511225053310324405552341300214450322545125330150043123141012421453202513005434045013322443102352331551412002403415510035111204255404452032"
    } else {
        panic!("Unsupported number of rooms: {}", n);
    };

    let plan = plan_str
        .chars()
        .map(|c| c.to_digit(10).unwrap() as usize)
        .collect::<Vec<_>>();

    //post-lightning refactor!!
    //let labels = judge.explore(&vec![plan.clone()])[0].clone();
    let labels =
        judge.explore(&vec![plan.iter().map(|&x| (None, x)).collect::<Vec<_>>()])[0].clone();
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
    V_map: Vec<Vec<Option<Lit>>>,
    // V_rows[i] = list of variables for time i.
    V_rows: Vec<Vec<Lit>>,
}
fn build_candidates(cnf: &mut Cnf, info: &PlanInfo, buckets: &Buckets) -> Candidates {
    let mut V_map = vec![vec![None; info.n]; info.m];
    let mut V_rows: Vec<Vec<Lit>> = vec![Vec::new(); info.m];
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

fn first_use_sbp_rect_truncated(cnf: &mut Cnf, W_full: &Vec<Vec<Lit>>) {
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

    let mut z = vec![vec![Lit::positive(0); m]; t];
    let mut p = vec![vec![Lit::positive(0); m]; t];
    for i in 0..t {
        for u in 0..m {
            z[i][u] = cnf.var();
            p[i][u] = cnf.var();
        }
    }
    for i in 0..t {
        for u in 0..m {
            cnf.inst.add_clause(clause![!W[i][u], p[i][u]]);
            cnf.inst.add_clause(clause![!z[i][u], W[i][u]]);
            cnf.inst.add_clause(clause![!z[i][u], p[i][u]]);
            if i == 0 {
                cnf.inst.add_clause(clause![!p[0][u], z[0][u]]);
                cnf.inst.add_clause(clause![!z[0][u], p[0][u]]);
            } else {
                cnf.inst.add_clause(clause![!p[i - 1][u], p[i][u]]);
                {
                    let c = vec![!p[i][u], p[i - 1][u], z[i][u]];
                    cnf.inst.add_clause(c.as_slice().into());
                }
                cnf.inst.add_clause(clause![!z[i][u], !p[i - 1][u]]);
            }
        }
    }
    for i in 0..t {
        for u in 1..m {
            cnf.inst.add_clause(clause![!p[i][u], p[i][u - 1]]);
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
        let mut W: Vec<Vec<Lit>> = Vec::with_capacity(times.len());
        for &i in times {
            let mut row = Vec::with_capacity(rooms.len());
            for &u in rooms {
                let var = cand.V_map[i][u].unwrap();
                row.push(var);
            }
            W.push(row);
        }
        // Anchor earliest occurrence to smallest room of this label
        cnf.add_unit(W[0][0]);
        first_use_sbp_rect_truncated(cnf, &W);
    }

    // Also pin the first seen time of each label to the canonical u=k if present.
    let mut seen = [false; 4];
    for i in 0..info.m {
        let k = info.labels[i];
        if !seen[k] {
            seen[k] = true;
            if let Some(v) = cand.V_map[i][k] {
                cnf.add_unit(v);
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
                    cnf.inst.add_clause(clause![!vi, !vj]);
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
                            let mut c = Vec::with_capacity(4);
                            c.push(!vi);
                            c.push(!vj);
                            c.push(!qi);
                            c.push(qj);
                            cnf.inst.add_clause(c.as_slice().into());
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
    Tlab: Vec<Vec<[Lit; 4]>>,
    // F[u][e][v]
    F: Vec<Vec<Vec<Lit>>>,
    // M[u][v][e][f] symmetric shared
    M: Vec<Vec<[[Lit; 6]; 6]>>,
}
fn build_edge_vars(cnf: &mut Cnf, info: &PlanInfo) -> EdgeVars {
    let n = info.n;
    let mut Tlab = vec![vec![[Lit::positive(0); 4]; 6]; n];
    let mut F = mat![Lit::positive(0); n; 6; n];

    for u in 0..n {
        for e in 0..6 {
            let mut trow = [Lit::positive(0); 4];
            for k in 0..4 {
                Tlab[u][e][k] = cnf.var();
                trow[k] = Tlab[u][e][k];
            }
            cnf.choose_one(&trow);

            let mut frow: Vec<Lit> = Vec::with_capacity(n);
            for v in 0..n {
                F[u][e][v] = cnf.var();
                frow.push(F[u][e][v]);
                cnf.inst.add_clause(clause![!F[u][e][v], Tlab[u][e][v % 4]]);
            }
            cnf.choose_one(&frow);
        }
    }

    for u in 0..n {
        for e in 0..6 {
            for k in 0..4 {
                cnf.buf.clear();
                cnf.buf.push(!Tlab[u][e][k]);
                for v in (k..n).step_by(4) {
                    cnf.buf.push(F[u][e][v]);
                }
                let clause_vec = cnf.buf.clone();
                cnf.inst.add_clause(clause_vec.as_slice().into());
            }
        }
    }

    let mut M = vec![vec![[[Lit::positive(0); 6]; 6]; n]; n];
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
                    cnf.inst.add_clause(clause![!mv, F[u][e][v]]);
                    cnf.inst.add_clause(clause![!mv, F[v][f][u]]);
                }
            }
        }
    }
    // Row-wise: F[u][e][v] -> OR_f M[u][v][e][f]; AMO on f
    for u in 0..n {
        for v in 0..n {
            for e in 0..6 {
                let mut row = [Lit::positive(0); 6];
                for f in 0..6 {
                    row[f] = M[u][v][e][f];
                }
                cnf.buf.clear();
                cnf.buf.push(!F[u][e][v]);
                cnf.buf.extend_from_slice(&row);
                let clause_vec = cnf.buf.clone();
                cnf.inst.add_clause(clause_vec.as_slice().into());
                amo_pairwise(&mut cnf.inst, &row);
            }
        }
    }
    // Column-wise: F[v][f][u] -> OR_e M[u][v][e][f]; AMO on e
    for u in 0..n {
        for v in 0..n {
            for f in 0..6 {
                let mut col = [Lit::positive(0); 6];
                for e in 0..6 {
                    col[e] = M[u][v][e][f];
                }
                cnf.buf.clear();
                cnf.buf.push(!F[v][f][u]);
                cnf.buf.extend_from_slice(&col);
                let clause_vec = cnf.buf.clone();
                cnf.inst.add_clause(clause_vec.as_slice().into());
                amo_pairwise(&mut cnf.inst, &col);
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
            cnf.inst.add_clause(clause![!vi, edges.Tlab[u][e][h]]);
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
                cnf.inst.add_clause(clause![!vi, !vj, edges.F[u][e][v]]);
            }
        }
    }
}

// -------------------------- Extraction -----------------------------------

fn lit_is_true(model: &Assignment, l: Lit) -> bool {
    let v = model.var_value(l.var());
    match (v, l.is_pos()) {
        (TernaryVal::True, true) => true,
        (TernaryVal::False, false) => true,
        _ => false,
    }
}

fn extract_guess(
    model: &Assignment,
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
            if lit_is_true(model, v) {
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
                if lit_is_true(model, edges.F[u][e][v]) {
                    v_sel = v;
                    break;
                }
            }
            let mut f_sel = 0usize;
            for f in 0..6 {
                if lit_is_true(model, edges.M[u][v_sel][e][f]) {
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

fn solve(judge: &mut dyn icfpc2025::judge::Judge) {
    // 1) Acquire single explore trace with a balanced, shuffled plan
    let info = acquire_plan_and_labels(judge);

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

    // 5) Solve with rustsat-kissat
    let cnf_c = cnf.inst.clone().into_cnf().0;
    let mut solver = rustsat_kissat::Kissat::default();
    solver.add_cnf(cnf_c).unwrap();
    let res = solver.solve().unwrap();
    assert!(matches!(res, SolverResult::Sat));
    let model = solver.full_solution().unwrap();

    // 6) Extract and verify
    let guess = extract_guess(&model, &info, &buckets, &cand, &edges);
    assert!(check_explore(
        &guess,
        &[info.plan.clone()],
        &[info.labels.clone()]
    ));
    judge.guess(&guess);
}
// EVOLVE-BLOCK-END

fn main() {
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    solve(judge.as_mut());
}
