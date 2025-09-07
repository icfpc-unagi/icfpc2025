// EVOLVE-BLOCK-START
#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
#![allow(non_snake_case)]
use icfpc2025::{judge::*, *};
use rand::prelude::*;
use rand_chacha::ChaCha12Rng;

// ----------------------------- CNF utilities -----------------------------

/// SAT変数IDを単調増加に発行するためのカウンタ。
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

/// At-Most-One制約をペアワイズ法でエンコードする場合の変数数の閾値。
const AMO_PAIRWISE_THRESHOLD: usize = 6;

/// At-Most-One制約をペアワイズ法でエンコードする。
/// 変数数が少ない場合に効率的。
#[inline]
fn amo_pairwise(sat: &mut cadical::Solver, xs: &[i32]) {
    for i in 0..xs.len() {
        for j in i + 1..xs.len() {
            sat.add_clause([-xs[i], -xs[j]]);
        }
    }
}

/// At-Most-One制約をシーケンシャルカウンタ法でエンコードする。
/// 変数数が多い場合に効率的。
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

/// Exactly-One制約をエンコードする。
/// At-Least-One節を追加した後、変数数に応じて最適なAt-Most-Oneエンコーディングを選択する。
#[inline]
fn choose_one(sat: &mut cadical::Solver, xs: &[i32], id: &mut Counter) {
    sat.add_clause(xs.iter().copied());
    if xs.len() <= AMO_PAIRWISE_THRESHOLD {
        amo_pairwise(sat, xs);
    } else {
        amo_sequential(sat, xs, id);
    }
}

/// CNF式を構築するためのヘルパー構造体。
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
    /// 新しいSAT変数を確保する。
    #[inline]
    fn var(&mut self) -> i32 {
        self.id.next()
    }
    /// 新しい節をCNFに追加する。
    #[inline]
    fn clause<I: IntoIterator<Item = i32>>(&mut self, lits: I) {
        self.sat.add_clause(lits);
    }
    /// Exactly-One制約をCNFに追加する。
    #[inline]
    fn choose_one(&mut self, xs: &[i32]) {
        choose_one(&mut self.sat, xs, &mut self.id);
    }
}

// -------------------------- Combinatorial helpers ------------------------

/// 2つの時刻i, jが観測系列から区別可能かどうかを判定する。
/// `diff[i][j] = true`は、時刻iとjの状態が異なることが確定していることを示す。
///
/// # 引数
/// * `plan` - 実行されたドアのシーケンス。
/// * `labels` - 各時刻で観測された部屋のラベル。
///
/// # 詳細
/// DPで計算する。`diff[i][j]`は以下の場合に`true`となる。
/// 1. `labels[i] != labels[j]`
/// 2. `labels[i] == labels[j]` かつ `plan[i] == plan[j]` かつ `diff[i+1][j+1]`
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
    // 対称性を保証する
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

/// 問題特有の情報を保持する構造体。
struct PlanInfo {
    /// 部屋数
    n: usize,
    plan: Vec<usize>,
    labels: Vec<usize>,
    /// planの長さ
    t: usize,
    /// labelsの長さ
    m: usize,
    /// 区別可能性テーブル
    diff: Vec<Vec<bool>>,
}

/// 均等にシャッフルされたバランスの取れた実行プランを生成する。
/// 各ドアを `18 * n / 6` 回ずつ使用する。
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

/// `explore` APIを一度だけ呼び出し、問題解決に必要な情報を収集する。
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
    PlanInfo {
        n,
        plan,
        labels,
        t,
        m,
        diff,
    }
}

/// 部屋と時刻をラベルごとに分類する。
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

/// 各時刻でどの部屋にいる可能性があるかを表すSAT変数を作成する。
struct Candidates {
    // V_map[i][u] = Some(var) if room u allowed at time i (label match).
    V_map: Vec<Vec<Option<i32>>>,
    // V_rows[i] = list of variables for time i.
    _V_rows: Vec<Vec<i32>>,
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
        // 各時刻でちょうど1つの部屋にいる、という制約を追加
        cnf.choose_one(&V_rows[i]);
    }
    Candidates {
        V_map,
        _V_rows: V_rows,
    }
}

// -------------------------- Symmetry breaking ----------------------------

/// First-Use Symmetry Breaking for Rectangular Matrices (Truncated)
/// W[t][m] (time x item) の行列に対し、アイテムが最初に使用される時刻に関する対称性を破る。
/// 探索空間を削減するため、t_all > m + 2 の場合に t = m + 2 に切り詰める。
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

/// 対称性を破るための制約を追加する。
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

/// `diff`テーブルに基づいた枝刈り制約を追加する。
/// 時刻iとjが区別可能で同じラベルを持つ場合、同じ部屋uにいることはできない。
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

/// グラフの辺に関する情報を表すSAT変数の集まり。
struct EdgeVars {
    /// Tlab[u][e][k]: 部屋uのドアeを通過した後の部屋のラベルがkであることを示す変数。
    Tlab: Vec<Vec<[i32; 4]>>,
    /// F[u][e][v]: 部屋uのドアeが部屋vに繋がっていることを示す変数。
    F: Vec<Vec<Vec<i32>>>,
    /// M[u][v][e][f]: 部屋uのドアeと部屋vのドアfが繋がっていることを示す変数。
    M: Vec<Vec<[[i32; 6]; 6]>>,
}

/// グラフの辺に関するSAT変数と制約を構築する。
fn build_edge_vars(cnf: &mut Cnf, info: &PlanInfo) -> EdgeVars {
    let n = info.n;
    let mut Tlab = vec![vec![[0i32; 4]; 6]; n];
    let mut F = mat![0; n; 6; n];

    // Tlab, F の変数を初期化し、基本的な制約を追加
    for u in 0..n {
        for e in 0..6 {
            // Tlab: ドアeの先のラベルはただ1つ
            let mut trow = [0i32; 4];
            for k in 0..4 {
                Tlab[u][e][k] = cnf.var();
                trow[k] = Tlab[u][e][k];
            }
            cnf.choose_one(&trow);

            // F: ドアeの先の部屋はただ1つ
            let mut frow = Vec::with_capacity(n);
            for v in 0..n {
                F[u][e][v] = cnf.var();
                frow.push(F[u][e][v]);
                // F[u][e][v]がtrueなら、Tlab[u][e][v%4]もtrueでなければならない
                cnf.clause([-F[u][e][v], Tlab[u][e][v % 4]]);
            }
            cnf.choose_one(&frow);
        }
    }

    // TlabとFの整合性制約
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

    // M: 辺のマッチング変数を初期化
    let mut M = vec![vec![[[0i32; 6]; 6]; n]; n];
    for u in 0..n {
        for v in u..n {
            for e in 0..6 {
                for f in 0..6 {
                    let var = cnf.var();
                    M[u][v][e][f] = var;
                    M[v][u][f][e] = var; // 対称性
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

/// 観測系列(plan)と状態変数(V)を辺変数と結びつける制約を追加する。
fn add_plan_constraints(
    cnf: &mut Cnf,
    info: &PlanInfo,
    buckets: &Buckets,
    cand: &Candidates,
    edges: &EdgeVars,
) {
    // V[i]=u (時刻iに部屋uにいる) -> Tlab[u, plan[i], labels[i+1]]
    // (i.e., ドアplan[i]を通過した先のラベルはlabels[i+1]である)
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
    // (i.e., 時刻iにu, i+1にvにいるなら、uのドアplan[i]はvに繋がっている)
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

/// SATソルバーの解からグラフ構造を復元する。
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

    // 部屋のラベルを割り当て
    for u in 0..n {
        guess.rooms[u] = u % 4;
    }

    // グラフの接続関係を復元
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

    // 5) Solve
    assert_eq!(cnf.sat.solve(), Some(true));

    // 6) Extract and verify
    let guess = extract_guess(&cnf, &info, &buckets, &cand, &edges);
    assert!(check_explore(
        &guess,
        &[info.plan.clone()],
        &[info.labels.clone()]
    ));
    panic!("Debug");
    judge.guess(&guess);
}
// EVOLVE-BLOCK-END

fn main() {
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    solve(judge.as_mut());
}
