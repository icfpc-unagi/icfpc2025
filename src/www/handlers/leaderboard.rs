//! # Leaderboard Page Handlers
//!
//! This module contains the handlers for rendering the leaderboard pages.
//! It fetches historical leaderboard data, visualizes it using Chart.js,
//! and displays the latest solved map for a given problem.

use crate::{api, problems, sql, svg};
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use cached::proc_macro::cached;
use chrono::NaiveDateTime;
use chrono_humanize::Humanize;
use mysql::params;
use serde::Deserialize;
use std::fmt::Write;
use tokio::time::Duration;

const _TZ: chrono::FixedOffset = chrono::FixedOffset::east_opt(9 * 3600).unwrap();

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default)]
    nocache: bool,
}

/// A helper to wrap content in the standard HTML page template.
fn html_page(title: &str, body: &str, banner: &str) -> String {
    // Auto-refresh leaderboard pages every 5 minutes
    let auto_refresh = "<script>setTimeout(() => location.reload(), 5*60*1000);</script>";
    crate::www::handlers::template::render(&format!(
        "{}<h1>{}</h1>\n{}\n{}",
        banner, title, auto_refresh, body
    ))
}

/// Renders the main leaderboard index page, which lists all available problems.
pub async fn index() -> impl Responder {
    let list = crate::problems::all_problems()
        .iter()
        .map(|p| {
            format!(
                "<li><a href=\"/leaderboard/{}\">{}</a> (size {})</li>",
                p.problem, p.problem, p.size
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let page = html_page("Leaderboards", &format!("<ul>{}</ul>", list), "");
    HttpResponse::Ok().content_type("text/html").body(page)
}

/// The path parameter for the `show` handler, capturing the problem name.
#[derive(Deserialize)]
pub struct ProblemPath {
    problem: String,
}

// Note: Previous implementation downsampled after download.
// We now downsample timestamps before download to reduce bandwidth.

/// The main handler for showing a leaderboard for a specific problem.
pub async fn show(
    path: web::Path<ProblemPath>,
    query: web::Query<LeaderboardQuery>,
) -> impl Responder {
    let problem = &path.problem;

    let result = async move { render_problem_leaderboard(problem, query.nocache).await };
    match result.await {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => crate::www::handlers::template::to_error_response(&e),
    }
}

/// The core logic for fetching data and rendering the leaderboard page for a single problem.
async fn render_problem_leaderboard(problem: &str, nocache: bool) -> Result<String> {
    // Fetch active lock
    // Build notification banner if active_lock_user exists
    let t0 = std::time::Instant::now();
    let banner_html = if let Some(user) = sql::cell::<String>(
        r"
          SELECT lock_user
          FROM locks
          WHERE lock_id = 1
          AND lock_expired > CURRENT_TIMESTAMP
          LIMIT 1
          ",
        params::Params::Empty,
    )? {
        format!(
            r#"<div style="width:100vw;position:relative;left:50%;right:50%;margin-left:-50vw;margin-right:-50vw;background-color:#66bb6a;color:white;font-weight:bold;padding:4px 0;text-align:center;font-size:2.4em;box-shadow:0 2px 8px rgba(0,0,0,0.08);z-index:1000;">
      <a href="/unlock"><img style="height:1em;vertical-align:text-bottom;" src="/static/sansho.png" alt="Lock icon">
      {user}
      🔒️</a>
      </div>"#
        )
    } else {
        String::new()
    };
    let banner_ms = t0.elapsed().as_millis();

    // Fetch all scores.
    let t0 = std::time::Instant::now();
    let scores = api::scores()?;
    let scores_ms = t0.elapsed().as_millis();

    // Build problem navigation links for the top of the page.
    // scores: Unagiのスコア一覧 (api::scores)
    // scoresテーブルから各問題ごとの全チーム最新スコアの最小値を取得
    let mut best_scores = std::collections::HashMap::new();
    let rows = sql::select(
        r#"
        SELECT problem, MIN(score) AS best_score
        FROM scores
        WHERE score IS NOT NULL
        GROUP BY problem
        "#,
        params::Params::Empty,
    )?;
    for row in rows {
        let problem = row.at::<String>(0)?;
        let best_score = row.at::<i64>(1)?;
        best_scores.insert(problem, best_score);
    }

    let mut nav_links: Vec<String> = Vec::new();
    if problem == "global" {
        nav_links.push("<b>[Global]</b>".to_string());
    } else {
        nav_links.push("[<a href=\"/leaderboard/global\">Global</a>]".to_string());
    }
    for problems::Problem { problem: p, .. } in problems::all_problems() {
        let score = scores.get(p);
        let best = best_scores.get(p);
        let mut link = format!(
            "[{}]({}/{})",
            p,
            score.map_or("-".to_string(), |s| s.to_string()),
            best.map_or("-".to_string(), |s| s.to_string())
        );
        // Unagiのスコアが最良でない場合は赤くする
        if let (Some(unagi_score), Some(best_score)) = (score, best)
            && unagi_score > best_score
        {
            link = format!(r#"<span style="color:red;">{}</span>"#, link);
        }
        if problem == p {
            link = format!("<b>{link}</b>");
        } else {
            link = format!(r#"<a href="/leaderboard/{p}">{link}</a>"#);
        }
        nav_links.push(link);
    }
    let nav_html = format!(
        "<div class=\"lb-nav\" style=\"margin:8px 0;\">{}</div>",
        nav_links.join(" ")
    );

    // Fetch recent guesses for the problem to display.
    let t0 = std::time::Instant::now();
    let guesses_html = recent_guesses(problem).await?;
    let recent_ms = t0.elapsed().as_millis();

    // Fetch the latest correct guess for the problem, optionally bypassing the cache.
    let t0 = std::time::Instant::now();
    let map_html = if problem == "global" {
        String::new()
    } else if nocache {
        last_correct_guess_prime_cache(problem)?
    } else {
        last_correct_guess(problem)?
    };
    let last_guess_ms = t0.elapsed().as_millis();

    let t0 = std::time::Instant::now();
    let snapshots = fetch_snapshots(problem).await?;
    let fetch_ms = t0.elapsed().as_millis();

    // Construct the final HTML page, embedding the data and the charting JavaScript.
    let html = format!(
        r#"
{nav}
<div>
  <h2>Problem: {problem}</h2>
  <p>Snapshots: {count}</p>
</div>
<div id="chart" style="width: 100%; height: 500px;"></div>
<div id="lb-table" style="margin-top:16px;"></div>
<script src="https://cdn.jsdelivr.net/npm/luxon@3/build/global/luxon.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
<script src="https://cdn.jsdelivr.net/npm/chartjs-adapter-luxon"></script>
<script>
const snapshots = {snapshots};
const problem = "{problem}";

// === Chart.js Data Preparation ===

// Parse our "YYYYMMDD-HHMMSS" timestamp strings into Date objects for Chart.js.
function parseTs(ts) {{
  const y = +ts.slice(0,4), mo = +ts.slice(4,6)-1, d = +ts.slice(6,8);
  const h = +ts.slice(9,11), mi = +ts.slice(11,13), s = +ts.slice(13,15);
  // Interpret original timestamp as UTC, then Chart.js adapter formats it in client's timezone.
  return new Date(Date.UTC(y, mo, d, h, mi, s));
}}
const labels = snapshots.map(s => parseTs(s.ts));

// Transform the snapshot data into a format Chart.js understands: one dataset per team.
// Each dataset is an array of scores, with `null` for timestamps where the team had no score.
const teamToData = new Map();
snapshots.forEach((snap, idx) => {{
  const arr = Array.isArray(snap.data) ? snap.data : [];
  for (const rec of arr) {{
    const team = rec.teamName;
    const score = rec.score;
    if (!team || score == null) continue;
    if (!teamToData.has(team)) teamToData.set(team, Array(labels.length).fill(null));
    teamToData.get(team)[idx] = +score;
  }}
}});

// Generate a consistent color for each team based on its name hash.
function colorFor(name) {{
  let h=0; for (let i=0;i<name.length;i++) h=(h*31+name.charCodeAt(i))>>>0;
  const hue=h%360; return `hsl(${{hue}} 70% 45%)`;
}}

// Create the dataset objects for Chart.js.
const datasets = Array.from(teamToData.entries()).map(([team, data]) => ({{
  label: team,
  data,
  borderColor: team === 'Unagi' ? '#e53935' : colorFor(team),
  backgroundColor: 'transparent',
  spanGaps: false,
  tension: 0.2,
  pointRadius: team === 'Unagi' ? 3 : 1,
  borderWidth: team === 'Unagi' ? 3 : 1,
}}));

// === Chart.js Rendering ===

const container = document.getElementById('chart');
const canvas = document.createElement('canvas');
container.appendChild(canvas);

const chart = new Chart(canvas.getContext('2d'), {{
  type: 'line',
  data: {{ labels, datasets }},
  options: {{
    responsive: true,
    maintainAspectRatio: false,
    interaction: {{ mode: 'nearest', intersect: false }},
    plugins: {{
      tooltip: {{ enabled: true }},
      legend: {{ display: false }}, // Legend is too crowded, use table instead.
    }},
    scales: {{
      x: {{ type: 'time', time: {{ unit: 'minute' }} }},
      // Use a logarithmic scale for scores, except for the global board.
      y: ((problem === 'global') ? {{ beginAtZero: true }} : {{ type: 'logarithmic' }}),
    }},
    adapters: {{
      date: {{ zone: 'Asia/Tokyo' }}, // Display times in JST.
    }},
  }},
}});

// === Leaderboard Table Generation ===

function esc(s) {{
  return String(s).replace(/[&<>"']/g, c => ({{
    '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;','\'':'&#39;'
  }})[c]);
}}
const latest = [];
for (const [team, data] of teamToData.entries()) {{
  let last = null;
  for (let i = data.length - 1; i >= 0; i--) {{
    if (data[i] != null) {{ last = data[i]; break; }}
  }}
  if (last == null) continue;
  latest.push({{ team, score: last }});
}}
// Sort by score (ascending for problems, descending for global).
if (problem === 'global') {{
  latest.sort((a,b) => b.score - a.score);
}} else {{
  latest.sort((a,b) => a.score - b.score);
}}
// Compute rows with tie-aware ranks.
let rows = '';
let lastScore = null;
let lastRank = 0;
latest.forEach((r, i) => {{
  const rank = (lastScore === r.score) ? lastRank : (i + 1);
  lastScore = r.score; lastRank = rank;
  // Skip zero scores.
  if (r.score == 0) return;
  const nameHtml = r.team === 'Unagi' ? `<strong>${{esc(r.team)}}</strong>` : esc(r.team);
  const teamAttr = esc(r.team);
  const nameLink = `<a href='#' data-team=\"${{teamAttr}}\">${{nameHtml}}</a>`;
  rows += `<tr>
    <td style=\"padding:4px 8px; text-align:right;\">${{rank}}</td>
    <td style=\"padding:4px 8px;\">${{nameLink}}</td>
    <td style=\"padding:4px 8px; text-align:right;\">${{r.score}}</td>
  </tr>`;
}});
document.getElementById('lb-table').innerHTML = `
  <table style="border-collapse:collapse; width:100%; font: 13px sans-serif;">
    <thead>
      <tr>
        <th style="text-align:right; padding:4px 8px;">Rank</th>
        <th style="text-align:left; padding:4px 8px;">Team</th>
        <th style="text-align:right; padding:4px 8px;">Score</th>
      </tr>
    </thead>
    <tbody>${{rows}}</tbody>
  </table>`;

// === Table/Chart Interactivity ===

let highlightedTeam = null;
// Toggles the highlighting of a team's series on the chart.
function highlightTeam(team) {{
  highlightedTeam = (highlightedTeam === team) ? null : team;
  chart.data.datasets.forEach(ds => {{
    const baseColor = ds.label === 'Unagi' ? '#e53935' : colorFor(ds.label);
    if (highlightedTeam && ds.label !== highlightedTeam) {{
      // Fade out non-highlighted teams.
      ds.borderColor = baseColor.startsWith('hsl(')
        ? baseColor.replace('hsl(', 'hsla(').replace(')', ', 0.2)')
        : (baseColor.length === 7 ? baseColor + '33' : baseColor);
      ds.borderWidth = 1;
      ds.pointRadius = 0;
    }} else {{
      // Emphasize the highlighted team (or all teams if none is highlighted).
      ds.borderColor = baseColor;
      ds.borderWidth = (ds.label === 'Unagi' || ds.label === highlightedTeam) ? 3 : 1;
      ds.pointRadius = (ds.label === 'Unagi' || ds.label === highlightedTeam) ? 3 : 1;
    }}
  }});
  chart.update();
}}

// Add a click listener to the table to handle highlighting.
document.getElementById('lb-table').addEventListener('click', (ev) => {{
  const a = ev.target.closest('a[data-team]');
  if (!a) return;
  ev.preventDefault();
  const team = a.getAttribute('data-team');
  highlightTeam(team);
}});
</script>
<h3>Recent guesses submitted</h3>
{guesses_html}
<h3>Latest successful map</h3>
{map_html}
"#,
        nav = nav_html,
        problem = problem,
        count = snapshots.len(),
        snapshots = serde_json::to_string(&snapshots)?,
    );
    // Append timing information at the end of the HTML body.
    let timings_html = format!(
        "\n<hr><div style=\"font:12px monospace;opacity:0.7;margin-top:8px;\">timings: fetch_snapshots={fetch_ms}ms, last_correct_guess={last_guess_ms}ms, recent_guesses={recent_ms}ms, banner_html={banner_ms}ms, api::scores={scores_ms}ms</div>",
    );
    let full_html = format!("{}{}", html, timings_html);

    Ok(html_page(
        &format!("Leaderboard - {problem}"),
        &full_html,
        &banner_html,
    ))
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct LeaderboardEntry {
    #[serde(rename = "teamName")]
    pub team_name: String,
    #[serde(rename = "teamPl", default)]
    pub team_pl: String,
    pub score: Option<i64>,
}
// Build JSON structure for the client side: [{ts, data: <json>}]
#[derive(serde::Serialize, Clone)]
struct Snapshot {
    ts: String,
    data: Vec<LeaderboardEntry>,
}

/// Fetches and downsamples leaderboard snapshots from GCS for a given problem.
#[cached(
    result = true,
    time = 60,
    key = "String",
    convert = "{problem.to_string()}",
    sync_writes = "by_key"
)]
async fn fetch_snapshots(problem: &str) -> Result<Vec<Snapshot>> {
    // scores テーブルから履歴取得
    let rows = sql::select(
        r#"
        SELECT timestamp, team_name, score
        FROM scores
        WHERE problem = :problem
        ORDER BY timestamp ASC
        "#,
        params! { "problem" => problem },
    )?;

    use std::collections::BTreeMap;
    let mut map: BTreeMap<chrono::NaiveDateTime, Vec<LeaderboardEntry>> = BTreeMap::new();
    for row in rows {
        let ts = row.at::<chrono::NaiveDateTime>(0)?;
        let team_name = row.at::<String>(1)?;
        let score = row.at::<i64>(2).ok();
        map.entry(ts).or_default().push(LeaderboardEntry {
            team_name,
            team_pl: String::new(), // 空文字で埋める
            score,
        });
    }
    // Downsample: 100件程度に間引き
    let keys: Vec<chrono::NaiveDateTime> = map.keys().cloned().collect();
    let n = keys.len();
    let keys = if n <= 100 {
        keys
    } else {
        let stride = n.div_ceil(100);
        let mut picked = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            if i % stride == 0 {
                picked.push(*k);
            }
        }
        if picked.last() != keys.last()
            && let Some(last) = keys.last() {
                picked.push(*last);
            }
        picked
    };
    let mut snapshots = Vec::new();
    for k in keys {
        let ts_str = k.format("%Y%m%d-%H%M%S").to_string();
        let data = map.get(&k).cloned().unwrap_or_default();
        snapshots.push(Snapshot { ts: ts_str, data });
    }
    Ok(snapshots)
}

/// 最近の提出（guess）を取得してHTMLとして返す関数
async fn recent_guesses(problem: &str) -> Result<String> {
    // 直近の提出（guess）を取得
    let rows = if problem != "global" {
        sql::select(
            "
        SELECT g.api_log_id AS id,
               g.api_log_created AS ts,
               s.api_log_request__problem_name AS problem,
               JSON_VALUE(g.api_log_response, '$.correct' RETURNING UNSIGNED) AS correct
        FROM api_logs g
        JOIN api_logs s
          ON g.api_log_select_id = s.api_log_id
            AND g.api_log_path = '/guess'
            AND s.api_log_path = '/select'
        WHERE s.api_log_request__problem_name = :problem
          AND g.api_log_response_code = 200
        ORDER BY g.api_log_id DESC
        LIMIT 20",
            params! { "problem" => problem },
        )?
    } else {
        sql::select(
            "
        SELECT g.api_log_id AS id,
               g.api_log_created AS ts,
               s.api_log_request__problem_name AS problem,
               JSON_VALUE(g.api_log_response, '$.correct' RETURNING UNSIGNED) AS correct
        FROM api_logs g
        JOIN api_logs s
          ON g.api_log_select_id = s.api_log_id
            AND g.api_log_path = '/guess'
            AND s.api_log_path = '/select'
        WHERE g.api_log_response_code = 200
        ORDER BY g.api_log_id DESC
        LIMIT 100",
            params::Params::Empty,
        )?
    };

    let mut w = String::new();
    w.push_str(
        r#"<table style="border-collapse:collapse;font-size:13px;">
        <tr><th>ID</th><th>Timestamp</th><th>Problem</th><th>Correct</th></tr>"#,
    );
    let now = chrono::Utc::now().naive_utc();
    for row in rows {
        let id = row.at::<i64>(0)?;
        let ts = row.at::<NaiveDateTime>(1)?;
        let problem = row.at::<String>(2)?;
        let correct = row.at::<bool>(3)?;
        write!(
            w,
            r#"<tr><td>{}</td><td title="{}">{}</td><td>{}</td><td>{}</td></tr>"#,
            id,
            ts.and_local_timezone(_TZ).unwrap().naive_local(),
            ts.signed_duration_since(now).humanize(),
            problem,
            if correct { "✅" } else { "❌" }
        )?;
    }
    w.push_str("</table>");
    Ok(w)
}

#[cached(
    result = true,
    key = "String",
    convert = "{problem.to_string()}",
    time = 300,
    sync_writes = "by_key"
)]
fn last_correct_guess(problem: &str) -> Result<String> {
    let mut w = String::new();
    if let Some(row) = sql::row(
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
        LIMIT 1",
        params! { "problem" => problem },
    )? {
        let api::GuessRequest { map, .. } = serde_json::from_str(&row.at::<String>(0)?)?;
        let n = map.rooms.len();
        write!(
            w,
            "<h4>Latest solved map (at {ts} UTC):</h4>",
            ts = row.at::<NaiveDateTime>(1)?,
        )?;

        // Data tables
        let mut doors = vec![[usize::MAX; 6]; n];
        let mut adj = vec![vec![0; n]; n];
        for api::MapConnection { from, to } in &map.connections {
            doors[from.room][from.door] = to.room;
            doors[to.room][to.door] = from.room;
            adj[from.room][to.room] += 1;
            adj[to.room][from.room] += 1;
        }
        write!(w, "<table><tr><th>d\\r")?;
        for j in 0..n {
            write!(w, "<th style=\"width:24px; text-align:center;\">{j}")?;
        }
        for i in 0..6 {
            write!(w, "<tr><th>{i}")?;
            for d in doors.iter() {
                write!(
                    w,
                    "<td style=\"background:#afa; text-align:center;\">{}",
                    d[i]
                )?;
            }
        }
        write!(w, "</table><table><tr><th>r\\r")?;
        for i in 0..n {
            write!(w, "<th style=\"width:24px; text-align:center;\">{i}")?;
        }
        for (i, row) in adj.iter().enumerate() {
            write!(w, "<tr><th style=\"width:24px; text-align:center;\">{i}")?;
            for (j, &val) in row.iter().enumerate() {
                write!(
                    w,
                    "<td style=\"background:{};text-align:center;\">{}",
                    if i == j { "#faa" } else { "#aaf" },
                    if val != 0 {
                        val.to_string()
                    } else {
                        String::new()
                    }
                )?;
            }
        }

        // Render d3 visualizer.
        let mut problem = serde_json::value::Map::new();
        problem.insert("map".to_string(), serde_json::to_value(&map)?);
        write!(
            w,
            r#"</table><div id="container"></div><script type="module">
              import chart from '/static/d3-visualizer.js';
              document.getElementById('container').append(chart({}));
            </script>"#,
            serde_json::to_string(&problem)?,
        )?;

        // Render the map as an SVG.
        write!(w, "{}", &svg::render(&map))?;
    } else {
        write!(w, "<div>No successful guess submitted</div>")?;
    }
    Ok(w)
}
