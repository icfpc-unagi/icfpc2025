use anyhow::{Context, Result, anyhow};
use icfpc2025::sql;
use mysql::params;
use serde::Serializer;
use serde::ser::SerializeMap;
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
        LIMIT 20
        "#,
        params! { "like_problem" => format!("%{}%", problem_name) },
    )?;

    let out_dir: PathBuf = ["localtest", "in", &problem_name].iter().collect();
    create_dir_all(&out_dir)?;

    for (i, row) in rows.iter().enumerate() {
        let cell: String = row.get("api_log_request")?;
        let v: Value = serde_json::from_str(&cell).with_context(|| {
            format!(
                "failed to parse api_log_request as JSON (index {}): {}",
                i, cell
            )
        })?;

        let path = out_dir.join(format!("{:03}.json", i));
        let file = File::create(&path)
            .with_context(|| format!("failed to create output file: {:?}", path))?;

        match v {
            Value::Object(mut map) => {
                // problemName first, then all other fields except id (and skipping existing problemName to avoid duplicates)
                write_object_with_problem_first(file, &problem_name, &mut map)?;
            }
            other => {
                // Wrap non-object into { problemName, data }
                let mut fmt = serde_json::Serializer::with_formatter(
                    file,
                    serde_json::ser::PrettyFormatter::with_indent(b"  "),
                );
                let mut m = fmt.serialize_map(None)?;
                m.serialize_entry("problemName", &problem_name)?;
                m.serialize_entry("data", &other)?;
                m.end()?;
                // Append newline
                // Serializer writes to the underlying writer directly; nothing to append here.
            }
        }
    }

    Ok(())
}

fn write_object_with_problem_first(
    mut w: File,
    problem_name: &str,
    map: &mut Map<String, Value>,
) -> Result<()> {
    let mut ser = serde_json::Serializer::with_formatter(
        &mut w,
        serde_json::ser::PrettyFormatter::with_indent(b"  "),
    );
    let mut m = ser.serialize_map(None)?;
    m.serialize_entry("problemName", problem_name)?;
    map.remove("id");
    if let Some(v) = map.remove("problemName") {
        // If original had problemName, we prefer our value; but keep original under data? Spec doesn't require; skip.
        let _ = v;
    }
    for (k, v) in map.iter() {
        m.serialize_entry(k, v)?;
    }
    m.end()?;
    // Ensure newline at EOF
    w.write_all(b"\n")?;
    Ok(())
}
