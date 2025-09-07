use actix_web::{Responder, web};
use anyhow::{Context, Result};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use mysql::params;

use crate::gcp::gcs::{download_object, get_object_metadata};
use crate::www::handlers::template;

#[derive(serde::Deserialize)]
pub struct TaskQuery {
    pub task_id: i64,
}

pub async fn show(query: web::Query<TaskQuery>) -> impl Responder {
    template::to_response(render_task_page(query.task_id).await)
}

async fn render_task_page(task_id: i64) -> Result<String> {
    // Fetch task row
    let row = crate::sql::row(
        r#"
        SELECT task_id, agent_id, problem_name, problem_variant,
               task_host, task_exit_code, task_score, task_duration_ms,
               task_lock, task_locked, task_failed, task_created, task_updated
        FROM tasks
        WHERE task_id = :task_id
        "#,
        params! { "task_id" => task_id },
    )?
    .context("task not found")?;

    // Extract fields
    let agent_id: i64 = row.get("agent_id")?;
    let problem_name: String = row.get("problem_name")?;
    let problem_variant: i64 = row.get("problem_variant")?;
    let task_host: Option<String> = row.get_option("task_host")?;
    let task_exit_code: Option<i64> = row.get_option("task_exit_code")?;
    let task_score: Option<i64> = row.get_option("task_score")?;
    let task_duration_ms: Option<i64> = row.get_option("task_duration_ms")?;
    let task_lock: Option<String> = row.get_option("task_lock")?;
    let task_locked: Option<NaiveDateTime> = row.get_option("task_locked")?;
    let task_failed: i64 = row.get("task_failed")?;
    let task_created: NaiveDateTime = row.get("task_created")?;
    let task_updated: NaiveDateTime = row.get("task_updated")?;

    // Build table
    let mut html = String::new();
    html.push_str(&format!("<h1>Task #{}</h1>", task_id));
    html.push_str("<table class=\"table\">\n");
    let add = |h: &mut String, k: &str, v: String| {
        h.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>\n",
            k,
            escape_html(&v)
        ));
    };
    add(&mut html, "task_id", format!("{}", task_id));
    add(
        &mut html,
        "agent_id",
        format!("<a href=\"/agent?agent_id={}\">{}</a>", agent_id, agent_id),
    );
    add(
        &mut html,
        "problem_name",
        format!(
            "<a href=\"/leaderboard/{}\">{}</a>",
            escape_attr(&problem_name),
            problem_name
        ),
    );
    add(&mut html, "problem_variant", format!("{}", problem_variant));
    add(&mut html, "task_host", task_host.unwrap_or_default());
    add(
        &mut html,
        "task_exit_code",
        task_exit_code.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(
        &mut html,
        "task_score",
        task_score.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(
        &mut html,
        "task_duration_ms",
        task_duration_ms.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(&mut html, "task_lock", task_lock.unwrap_or_default());
    add(&mut html, "task_locked", fmt_jst_opt(task_locked));
    add(&mut html, "task_failed", format!("{}", task_failed));
    add(&mut html, "task_created", fmt_jst(task_created));
    add(&mut html, "task_updated", fmt_jst(task_updated));
    html.push_str("</table>\n");

    // Logs from GCS
    let bucket = "icfpc2025-data";
    let stdout_name = format!("logs/{}/stdout.jsonl", task_id);
    let stderr_name = format!("logs/{}/stderr.jsonl", task_id);

    // Attempt metadata to check existence (optional)
    let _ = get_object_metadata(bucket, &stdout_name).await;
    let _ = get_object_metadata(bucket, &stderr_name).await;
    let stdout_bytes = download_object(bucket, &stdout_name)
        .await
        .unwrap_or_default();
    let stderr_bytes = download_object(bucket, &stderr_name)
        .await
        .unwrap_or_default();

    let (out_head, out_tail) = split_head_tail(&stdout_bytes, 500 * 1024);
    let (err_head, err_tail) = split_head_tail(&stderr_bytes, 500 * 1024);

    html.push_str("<h2>stdout (first 500KB)</h2><pre><code>");
    html.push_str(&escape_html(&String::from_utf8_lossy(out_head)));
    html.push_str("</code></pre>");
    html.push_str("<h2>stdout (last 500KB)</h2><pre><code>");
    html.push_str(&escape_html(&String::from_utf8_lossy(out_tail)));
    html.push_str("</code></pre>");
    html.push_str("<h2>stderr (first 500KB)</h2><pre><code>");
    html.push_str(&escape_html(&String::from_utf8_lossy(err_head)));
    html.push_str("</code></pre>");
    html.push_str("<h2>stderr (last 500KB)</h2><pre><code>");
    html.push_str(&escape_html(&String::from_utf8_lossy(err_tail)));
    html.push_str("</code></pre>");

    Ok(template::render(&html))
}

fn fmt_jst_opt(dt: Option<NaiveDateTime>) -> String {
    dt.map(fmt_jst).unwrap_or_default()
}

fn fmt_jst(dt: NaiveDateTime) -> String {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    jst.from_utc_datetime(&dt)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
}

fn split_head_tail(bytes: &[u8], limit: usize) -> (&[u8], &[u8]) {
    let len = bytes.len();
    let head_len = limit.min(len);
    let tail_len = limit.min(len);
    let head = &bytes[..head_len];
    let tail = &bytes[len.saturating_sub(tail_len)..];
    (head, tail)
}

fn escape_html(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            '/' => "&#x2F;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
