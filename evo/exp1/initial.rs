use rand::prelude::*;

// EVOLVE-BLOCK-START
fn solve(judge: &mut dyn icfpc2025::judge::Judge) {
    let num_rooms = judge.num_rooms();

    // Generate a random-walk plan
    let q = num_rooms * 18;
    let mut rnd = rand::rng();
    let mut plan = Vec::with_capacity(q);
    for _ in 0..q {
        let c: usize = rnd.random_range(0..6);
        plan.push(c);
    }
    let plans = vec![plan];
    let _ = judge.explore(&plans);

    let out = icfpc2025::judge::Guess {
        rooms: vec![0; num_rooms],
        start: 0,
        graph: vec![[(0, 0); 6]; num_rooms],
    };
    judge.guess(&out);
}
// EVOLVE-BLOCK-END

fn main() {
    let mut judge = icfpc2025::judge::get_judge_from_stdin();
    solve(judge.as_mut());
}
