use anyhow::Context as _;
use chrono::NaiveDateTime;
use icfpc2025::sql;
use mysql::params;

fn main() -> anyhow::Result<()> {
    let problem = std::env::args().nth(1).context("Usage: tos3 <problem>")?;
    let rows = sql::select(
        "
        SELECT g.api_log_request AS guess,
               g.api_log_created AS ts
        FROM api_logs g
        JOIN api_logs s
          ON g.api_log_select_id = s.api_log_id
            AND g.api_log_path = '/guess'
            AND s.api_log_path = '/select'
        WHERE s.api_log_request__problem_name = :problem
          AND g.api_log_response_code = 200
          AND JSON_EXTRACT(g.api_log_response, '$.correct') = true
        ORDER BY g.api_log_id DESC
        LIMIT 10",
        params! { "problem" => problem },
    )?;

    for row in rows {
        let guess = row.at::<String>(0)?;
        let ts = row.at::<NaiveDateTime>(1)?;
        // let api::GuessRequest { map, .. } = serde_json::from_str(&guess)?;
        // let n = map.rooms.len();
        // write!(
        //     w,
        //     "<h4>Latest solved map (at {ts} UTC):</h4>",
        //     ts = row.at::<NaiveDateTime>(1)?,
        // )?;
        println!("{ts}: {guess}");
    }
    Ok(())
}
