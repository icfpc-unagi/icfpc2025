use actix_web::{Responder, web};
use anyhow::Result;
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use mysql::params;

use crate::www::handlers::template;

#[derive(serde::Deserialize, Default)]
pub struct TasksQuery {
    #[serde(default = "default_page")] // 1-based page index
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

pub async fn index(query: web::Query<TasksQuery>) -> impl Responder {
    template::to_response(render_tasks_page(query.page).await)
}

async fn render_tasks_page(page: i64) -> Result<String> {
    let page = if page < 1 { 1 } else { page };
    let limit: i64 = 100; // fixed as requested
    let offset: i64 = (page - 1) * limit;

    let rows = crate::sql::select(
        r#"
        SELECT
            t.task_id,
            a.agent_name,
            t.problem_name,
            t.problem_variant,
            t.task_score,
            t.task_exit_code,
            t.task_locked,
            t.task_updated,
            CASE
              WHEN t.task_exit_code IS NULL THEN
                CASE WHEN t.task_locked > CURRENT_TIMESTAMP THEN '実行中' ELSE '待機中' END
              ELSE
                CASE WHEN t.task_exit_code = 0 THEN '成功' ELSE CONCAT('失敗(', t.task_exit_code, ')') END
            END AS task_status
        FROM tasks t
        LEFT JOIN agents a ON a.agent_id = t.agent_id
        ORDER BY t.task_id DESC
        LIMIT :limit_plus_one OFFSET :offset
        "#,
        params! { "limit_plus_one" => (limit + 1), "offset" => offset },
    )?;

    let mut items: Vec<TaskRow> = Vec::with_capacity(rows.len().min(limit as usize));
    for r in rows.iter().take(limit as usize) {
        let task_id: i64 = r.get("task_id")?;
        let agent_name: Option<String> = r.get_option("agent_name")?;
        let problem_name: String = r.get("problem_name")?;
        let problem_variant: i64 = r.get("problem_variant")?;
        let task_score: Option<i64> = r.get_option("task_score")?;
        let task_status: String = r.get("task_status")?;
        let task_updated: NaiveDateTime = r.get("task_updated")?;
        items.push(TaskRow {
            task_id,
            agent_name: agent_name.unwrap_or_else(|| "(unknown)".to_string()),
            problem_name,
            problem_variant,
            task_score,
            task_status,
            task_updated,
        });
    }
    let has_next = rows.len() as i64 > limit;

    // Render HTML
    let mut html = String::new();
    html.push_str("<h1>タスク一覧</h1>\n");
    html.push_str("<table class=\"table\">\n");
    html.push_str(
        "<tr><th>タスクID</th><th>プログラム名</th><th>問題名（問題シード）</th><th>スコア</th><th>ステータス</th><th>更新時刻</th></tr>\n",
    );
    for it in items {
        let id_html = format!(
            "<a href=\"/task?task_id={}\">{}</a>",
            it.task_id, it.task_id
        );
        let prob = format!("{} ({})", escape_html(&it.problem_name), it.problem_variant);
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            id_html,
            escape_html(&it.agent_name),
            prob,
            it.task_score.map(|v| v.to_string()).unwrap_or_default(),
            escape_html(&it.task_status),
            escape_html(&fmt_jst(it.task_updated)),
        ));
    }
    html.push_str("</table>\n");

    // Pagination
    html.push_str("<div class=\"pager\">");
    if page > 1 {
        html.push_str(&format!(
            "<a href=\"/tasks?page={}\">&laquo; 前のページ</a>",
            page - 1
        ));
    }
    if has_next {
        if page > 1 {
            html.push_str(" &nbsp;| &nbsp;");
        }
        html.push_str(&format!(
            "<a href=\"/tasks?page={}\">次のページ &raquo;</a>",
            page + 1
        ));
    }
    html.push_str("</div>");

    Ok(html)
}

struct TaskRow {
    task_id: i64,
    agent_name: String,
    problem_name: String,
    problem_variant: i64,
    task_score: Option<i64>,
    task_status: String,
    task_updated: NaiveDateTime,
}

fn fmt_jst(dt: NaiveDateTime) -> String {
    let jst = FixedOffset::east_opt(9 * 3600).unwrap();
    jst.from_utc_datetime(&dt)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
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
