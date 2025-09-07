use anyhow::{Context, Result, anyhow};
use icfpc2025::sql;
use mysql::params;
use serde::ser::Serialize;
use serde_json::{Map, Value};
use std::env;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("fetch_problems error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let problem_name = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("usage: fetch_problems <problem_name>"))?;

    let rows = sql::select(
        r#"
        SELECT a.api_log_request
        FROM api_logs AS a
        JOIN api_logs AS b ON a.api_log_select_id = b.api_log_id
        WHERE a.api_log_path = "/guess"
          AND b.api_log_path = "/select"
          AND a.api_log_response LIKE "%true%"
          AND b.api_log_response LIKE :like_problem
        ORDER BY a.api_log_id
        LIMIT 10
        "#,
        params! { "like_problem" => format!("%{}%", problem_name) },
    )?;

    let out_dir: PathBuf = ["localtest", "in", &problem_name].iter().collect();
    create_dir_all(&out_dir)?;

    for (i, row) in rows.iter().enumerate() {
        let cell: String = row.get("api_log_request")?;
        let mut v: Value = serde_json::from_str(&cell).with_context(|| {
            format!(
                "failed to parse api_log_request as JSON (index {}): {}",
                i, cell
            )
        })?;

        // Expect an object; mutate by removing id and inserting problemName first
        let mut new_obj = Map::new();
        new_obj.insert(
            "problemName".to_string(),
            Value::String(problem_name.clone()),
        );

        match v {
            Value::Object(mut map) => {
                map.remove("id");
                for (k, vv) in map.into_iter() {
                    new_obj.insert(k, vv);
                }
                v = Value::Object(new_obj);
            }
            _ => {
                // Not an object; still wrap it with problemName and original under "data"
                new_obj.insert("data".to_string(), v);
                v = Value::Object(new_obj);
            }
        }

        let path = out_dir.join(format!("{:03}.json", i));
        let file = File::create(&path)
            .with_context(|| format!("failed to create output file: {:?}", path))?;
        write_pretty_2space(file, &v)?;
    }

    Ok(())
}

fn write_pretty_2space(mut w: File, v: &Value) -> Result<()> {
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"  ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser)?;
    buf.push(b'\n');
    w.write_all(&buf)?;
    Ok(())
}
