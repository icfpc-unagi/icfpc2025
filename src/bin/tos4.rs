use std::io::Write as _;

use anyhow::Result;
use icfpc2025::judge::LocalJudge;

fn main() -> Result<()> {
    let mut file = std::fs::File::create("3x6.jsonl")?;
    let num_rooms = 3 * 6;
    for seed in 0..24 {
        let LocalJudge {
            starting_room,
            rooms,
            graph,
            ..
        } = LocalJudge::new("random_3layers", num_rooms, seed);
        let output =
            icfpc2025::layered::reduce_graph_without_to_door(starting_room, rooms, graph).unwrap();
        let json_out = serde_json::to_string(&output).unwrap();
        writeln!(file, "{}", json_out)?;
    }
    Ok(())
}
