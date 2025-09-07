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
        SELECT t.task_id, t.agent_id, a.agent_name, a.agent_bin, a.agent_code,
               t.problem_name, t.problem_variant,
               t.task_host, t.task_exit_code, t.task_score, t.task_duration_ms,
               t.task_lock, t.task_locked, t.task_failed, t.task_created, t.task_updated
        FROM tasks t
        LEFT JOIN agents a ON a.agent_id = t.agent_id
        WHERE t.task_id = :task_id
        "#,
        params! { "task_id" => task_id },
    )?
    .context("task not found")?;

    // Extract fields
    let agent_id: i64 = row.get("agent_id")?;
    let agent_name: Option<String> = row.get_option("agent_name")?;
    let agent_bin: Option<String> = row.get_option("agent_bin")?;
    let agent_code: Option<String> = row.get_option("agent_code")?;
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
    html.push_str(&format!("<h1>Task #{} </h1>", task_id));
    html.push_str("<table class=\"table\">\n");
    // add: escapes value for safe text rendering
    let add = |h: &mut String, k: &str, v: String| {
        h.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>\n",
            k,
            escape_html(&v)
        ));
    };
    // add_raw: value is inserted as raw HTML (caller must escape as appropriate)
    let add_raw = |h: &mut String, k: &str, v_html: String| {
        h.push_str(&format!("<tr><th>{}</th><td>{}</td></tr>\n", k, v_html));
    };

    // タスクID
    add(&mut html, "タスクID", format!("{}", task_id));

    // プログラム名 (ID) — plain text (no /agent page)
    let agent_name_text = agent_name.unwrap_or_else(|| "(unknown)".to_string());
    add(
        &mut html,
        "プログラム名 (ID)",
        format!("{} ({})", agent_name_text, agent_id),
    );

    // 問題（リンク表示）
    let problem_link = format!(
        "<a href=\"/leaderboard/{}\">{}</a>",
        escape_attr(&problem_name),
        escape_html(&problem_name)
    );
    add_raw(&mut html, "問題", problem_link);

    // 問題シード
    add(&mut html, "問題シード", format!("{}", problem_variant));

    // 実行ホスト名
    add(&mut html, "実行ホスト名", task_host.unwrap_or_default());
    // 実行バイナリ
    add(&mut html, "実行バイナリ", agent_bin.unwrap_or_default());
    add(
        &mut html,
        "終了コード",
        task_exit_code.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(
        &mut html,
        "スコア",
        task_score.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(
        &mut html,
        "実行時間",
        task_duration_ms.map(|v| v.to_string()).unwrap_or_default(),
    );
    add(&mut html, "ロック署名", task_lock.unwrap_or_default());
    add(&mut html, "ロック期限", fmt_jst_opt(task_locked));
    add(&mut html, "失敗回数", format!("{}", task_failed));
    add(&mut html, "作成時刻", fmt_jst(task_created));
    add(&mut html, "更新時刻", fmt_jst(task_updated));
    html.push_str("</table>\n");

    // 実行コード（複数行のシェルスクリプト）
    if let Some(code) = agent_code {
        if !code.is_empty() {
            html.push_str("<h2>実行コード</h2><pre><code>");
            html.push_str(&escape_html(&code));
            html.push_str("</code></pre>");
        }
    }

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

    // Parse JSONL and render combined output with omission markers
    let stdout_text = jsonl_to_text(&stdout_bytes);
    let stderr_text = jsonl_to_text(&stderr_bytes);
    let out_render = render_with_omission(&stdout_text, 500 * 1024, 500 * 1024);
    let err_render = render_with_omission(&stderr_text, 500 * 1024, 500 * 1024);

    html.push_str("<h2>標準出力</h2><pre><code>");
    html.push_str(&escape_html(&out_render));
    html.push_str("</code></pre>");

    html.push_str("<h2>標準エラー</h2><pre><code>");
    html.push_str(&escape_html(&err_render));
    html.push_str("</code></pre>");

    Ok(html)
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

fn jsonl_to_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let s = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                out.push_str(text);
            } else if let Some(tr) = v.get("truncated").and_then(|t| t.as_u64()) {
                // Indicate omission from the logger itself
                out.push_str("\n… ログの一部が省略されました (");
                out.push_str(&tr.to_string());
                out.push_str(" bytes) …\n");
            }
        }
    }
    out
}

fn render_with_omission(text: &str, head_bytes: usize, tail_bytes: usize) -> String {
    let len = text.len();
    if len <= head_bytes {
        return text.to_string();
    }
    if len <= head_bytes + tail_bytes {
        return text.to_string();
    }
    // Safe UTF-8 boundaries
    let mut head_end = head_bytes.min(len);
    while head_end > 0 && !text.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut tail_start = len.saturating_sub(tail_bytes);
    while tail_start < len && !text.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    if head_end >= tail_start {
        // Overlap; show whole to avoid duplication
        return text.to_string();
    }
    let mut out = String::with_capacity(head_end + 64 + (len - tail_start));
    out.push_str(&text[..head_end]);
    out.push_str("\n… 中略 …\n");
    out.push_str(&text[tail_start..]);
    out
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
