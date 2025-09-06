/*
use icfpc2025::judge::*;
use rand::prelude::*;
use z3::{ast::Bool, ast::Int, SatResult, Solver};

fn main() {
    let mut judge = get_judge_from_stdin();
    let mut rnd = rand::rng();

    let n = judge.num_rooms();

    // Generate a random plan of length n*18 with doors 0..5
    let mut plan = vec![];
    for _ in 0..(n * 18) {
        let c: usize = rnd.random_range(0..6);
        plan.push(c);
    }
    let r = judge.explore(&vec![plan.clone()]);

    assert_eq!(r.len(), 1);
    let seq = &r[0];
    let q = seq.len();

    // Z3 setup (implicit thread-local context). We rely on defaults.
    let solver = Solver::new();

    // Helper to name variables
    let v_name = |i: usize, u: usize| format!("V_{}_{}", i, u);
    let l_name = |u: usize, k: usize| format!("L_{}_{}", u, k);
    let d_name = |u: usize, e: usize| format!("D_{}_{}", u, e); // destination room index for (u,e)
    let f_name = |u: usize, e: usize| format!("F_{}_{}", u, e); // return door index for (u,e)

    // Variables
    // V[i][u]: Bool - step i is at room u
    let v_vars: Vec<Vec<Bool>> = (0..q)
        .map(|i| {
            (0..n)
                .map(|u| Bool::new_const(v_name(i, u)))
                .collect()
        })
        .collect();

    // L[u][k]: Bool - room u has label k (0..3)
    let l_vars: Vec<Vec<Bool>> =
        (0..n).map(|u| (0..4).map(|k| Bool::new_const(l_name(u, k))).collect()).collect();

    // Encode graph using Int variables instead of 4D Bool array for efficiency
    // D[u][e] in [0, n-1], F[u][e] in [0, 5]
    let d_vars: Vec<Vec<Int>> =
        (0..n).map(|u| (0..6).map(|e| Int::new_const(d_name(u, e))).collect()).collect();
    let f_vars: Vec<Vec<Int>> =
        (0..n).map(|u| (0..6).map(|e| Int::new_const(f_name(u, e))).collect()).collect();

    // Convenience constructors
    let int = |x: i64| Int::from_i64(x);

    // Constraints
    // 1) For each i, exactly one u: V[i][u]
    for i in 0..q {
        // at least one
        let ors: Vec<&Bool> = v_vars[i].iter().collect();
        solver.assert(&Bool::or(&ors));
        // at most one (pairwise)
        for u in 0..n {
            for v in (u + 1)..n {
                solver.assert(&Bool::or(&[&v_vars[i][u].not(), &v_vars[i][v].not()]));
            }
        }
    }

    // 2) For each room u, exactly one label k: L[u][k]
    for u in 0..n {
        let ors: Vec<&Bool> = l_vars[u].iter().collect();
        solver.assert(&Bool::or(&ors));
        for k in 0..4 {
            for l in (k + 1)..4 {
                solver.assert(&Bool::or(&[&l_vars[u][k].not(), &l_vars[u][l].not()]));
            }
        }
    }

    // 3) Domain constraints for D/F
    for u in 0..n {
        for e in 0..6 {
            solver.assert(&d_vars[u][e].ge(&int(0)));
            solver.assert(&d_vars[u][e].le(&int((n as i64) - 1)));
            solver.assert(&f_vars[u][e].ge(&int(0)));
            solver.assert(&f_vars[u][e].le(&int(5)));
        }
    }

    // 4) If step i is at room u then its label matches seq[i]
    for i in 0..q {
        for u in 0..n {
            let li = &l_vars[u][seq[i]];
            solver.assert(&v_vars[i][u].implies(li));
        }
    }

    // 5) Transitions along the plan: if at u at step i and at v at step i+1,
    //    then D[u][plan[i]] == v (there exists an f implicitly via F)
    for i in 0..(q - 1) {
        let e = plan[i];
        for u in 0..n {
            for v in 0..n {
                let cond = Bool::and(&[&v_vars[i][u], &v_vars[i + 1][v]]);
                solver.assert(&cond.implies(&d_vars[u][e].eq(&int(v as i64))));
            }
        }
    }

    // 6) Undirected graph constraint: following an edge and then its return door goes back
    for u in 0..n {
        for e in 0..6 {
            let v = &d_vars[u][e];
            let f = &f_vars[u][e];
            // D[D[u][e]][F[u][e]] == u
            // F[D[u][e]][F[u][e]] == e
            // Since indices are Ints, we encode by case-splitting on all possibilities for (v,f)
            // to keep it simple and robust.
            let mut cases_back: Vec<Bool> = vec![];
            let mut cases_door: Vec<Bool> = vec![];
            for vv in 0..n {
                for ff in 0..6 {
                    let v_is = v.eq(&int(vv as i64));
                    let f_is = f.eq(&int(ff as i64));
                    let both = Bool::and(&[&v_is, &f_is]);
                    let back_ok = d_vars[vv][ff].eq(&int(u as i64));
                    let door_ok = f_vars[vv][ff].eq(&int(e as i64));
                    cases_back.push(both.implies(&back_ok));
                    cases_door.push(both.implies(&door_ok));
                }
            }
            let back_refs: Vec<&Bool> = cases_back.iter().collect();
            let door_refs: Vec<&Bool> = cases_door.iter().collect();
            solver.assert(&Bool::and(&back_refs));
            solver.assert(&Bool::and(&door_refs));
        }
    }

    // Solve
    match solver.check() {
        SatResult::Sat => {}
        SatResult::Unsat => panic!("unsat"),
        SatResult::Unknown => panic!("unknown"),
    }
    let model = solver.get_model().expect("no model");

    // Decode rooms
    let mut rooms = vec![0usize; n];
    for u in 0..n {
        let mut found = false;
        for k in 0..4 {
            let val = model
                .eval(&l_vars[u][k], true)
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            if val {
                rooms[u] = k;
                found = true;
                break;
            }
        }
        if !found {
            panic!("Room label not found for {}", u);
        }
    }

    // Decode start
    let mut start = None;
    for u in 0..n {
        let val = model
            .eval(&v_vars[0][u], true)
            .and_then(|b| b.as_bool())
            .unwrap_or(false);
        if val {
            start = Some(u);
            break;
        }
    }
    let start = start.expect("No start room found");

    // Decode graph from D/F
    let mut graph = vec![[(0usize, 0usize); 6]; n];
    for u in 0..n {
        for e in 0..6 {
            let v = model
                .eval(&d_vars[u][e], true)
                .and_then(|x| x.as_i64())
                .expect("No value for D");
            let f = model
                .eval(&f_vars[u][e], true)
                .and_then(|x| x.as_i64())
                .expect("No value for F");
            graph[u][e] = (v as usize, f as usize);
        }
    }

    judge.guess(&Guess {
        start,
        rooms,
        graph,
    });
}
*/
