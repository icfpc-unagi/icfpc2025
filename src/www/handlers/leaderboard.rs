use crate::{api, svg};
use crate::{api::MapConnection, sql};
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::NaiveDateTime;
use mysql::params;
use serde::Deserialize;
use std::fmt::Write;

fn html_page(title: &str, body: &str) -> String {
    // Auto-refresh leaderboard pages every 5 minutes
    let auto_refresh = "<script>setTimeout(() => location.reload(), 5*60*1000);</script>";
    crate::www::handlers::template::render(&format!(
        "<h1>{}</h1>\n{}\n{}",
        title, auto_refresh, body
    ))
}

pub async fn index() -> impl Responder {
    let list = crate::problems::all_problems()
        .iter()
        .map(|p| {
            format!(
                "<li><a href=\"/leaderboard/{}\">{}</a> (size {})</li>",
                p.problem_name, p.problem_name, p.size
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let page = html_page("Leaderboards", &format!("<ul>{}</ul>", list));
    HttpResponse::Ok().content_type("text/html").body(page)
}

#[derive(Deserialize)]
pub struct ProblemPath {
    problem: String,
}

fn downsample<T: Clone>(v: &[(String, T)], max_points: usize) -> Vec<(String, T)> {
    if v.len() <= max_points {
        return v.to_vec();
    }
    let n = v.len();
    let stride = n.div_ceil(max_points);
    let mut out = Vec::new();
    for (i, item) in v.iter().enumerate() {
        if i % stride == 0 {
            out.push(item.clone());
        }
    }
    if out.last().map(|x| &x.0) != v.last().map(|x| &x.0) {
        out.push(v.last().unwrap().clone());
    }
    out
}

pub async fn show(path: web::Path<ProblemPath>) -> impl Responder {
    let problem = &path.problem;
    let bucket = "icfpc2025-data";

    let result = async move { render_problem_leaderboard(bucket, problem).await };
    match result.await {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => crate::www::handlers::template::to_error_response(&e),
    }
}

async fn render_problem_leaderboard(bucket: &str, problem: &str) -> Result<String> {
    // Build problem navigation links
    let mut nav_links: Vec<String> = Vec::new();
    nav_links.push("[<a href=\"/leaderboard/global\">Global</a>]".to_string());
    for p in crate::problems::all_problems() {
        nav_links.push(format!(
            "[<a href=\"/leaderboard/{}\">{}</a>]",
            p.problem_name, p.problem_name
        ));
    }
    let nav_html = format!(
        "<div class=\"lb-nav\" style=\"margin:8px 0;\">{}</div>",
        nav_links.join(" ")
    );

    // Fetch latest successful map for this problem
    let mut map_html = String::new();
    if let Some(row) = sql::row(
        "
        SELECT JSON_EXTRACT(g.api_log_request, '$.map') AS map,
               g.api_log_created AS ts
        FROM api_logs s
        JOIN api_logs g
          ON g.api_log_path = '/guess'
            AND g.api_log_id > s.api_log_id
        WHERE s.api_log_path = '/select'
          AND JSON_EXTRACT(s.api_log_request, '$.problemName') = :problem
          AND g.api_log_response_code = 200
          AND JSON_EXTRACT(g.api_log_response, '$.correct') = true",
        params! { "problem" => problem },
    )? {
        let map: api::Map = serde_json::from_str(&row.at::<String>(0)?)?;
        let n = map.rooms.len();
        let mut doors = vec![[usize::MAX; 6]; n];
        let mut adj = vec![vec![0; n]; n];
        for MapConnection { from, to } in &map.connections {
            doors[from.room][from.door] = to.room;
            doors[to.room][to.door] = from.room;
            adj[from.room][to.room] += 1;
            adj[to.room][from.room] += 1;
        }
        let mut w = &mut map_html;
        write!(w, "<table><tr><th>d\\r")?;
        for j in 0..n {
            write!(w, "<th style=\"width:24px; text-align:center;\">{j}")?;
        }
        for i in 0..6 {
            write!(w, "<tr><td>{i}")?;
            for j in 0..n {
                write!(
                    w,
                    "<td style=\"background:#afa; text-align:center;\">{}",
                    doors[j][i]
                )?;
            }
        }
        write!(w, "</table><table><tr><th>r\\r")?;
        for i in 0..n {
            write!(w, "<th style=\"width:24px; text-align:center;\">{i}")?;
        }
        for i in 0..n {
            write!(w, "<tr><td style=\"width:24px; text-align:center;\">{i}")?;
            for j in 0..n {
                write!(
                    w,
                    "<td style=\"background:#aaf; text-align:center;\">{}",
                    adj[i][j]
                )?;
            }
        }
        write!(
            w,
            "</table><div>Latest solved map (at {ts} UTC): {svg}</div>",
            ts = row.at::<NaiveDateTime>(1)?,
            svg = svg::render(&map),
        )?;
    } else {
        write!(map_html, "<div>No successful guess submitted</div>")?;
    }

    // List timestamps under history/
    let (dirs, _files) = crate::gcp::gcs::list_dir(bucket, "history").await?;
    // dirs like "YYYYMMDD-hhmmss/"; normalize and sort
    let mut stamps: Vec<String> = dirs
        .into_iter()
        .map(|d| d.trim_end_matches('/').to_string())
        .collect();
    stamps.sort();

    // Fetch all snapshots in parallel
    let mut set = tokio::task::JoinSet::new();
    for ts in stamps {
        let object = format!("history/{}/{}.json", ts, problem);
        let b = bucket.to_string();
        let ts_clone = object.clone();
        set.spawn(async move {
            match crate::gcp::gcs::download_object(&b, &object).await {
                Ok(bytes) => Ok((ts_clone, bytes)),
                Err(_e) => Err(()),
            }
        });
    }
    let mut snaps: Vec<(String, Vec<u8>)> = Vec::new();
    while let Some(res) = set.join_next().await {
        if let Ok(Ok((object_path, bytes))) = res {
            // extract timestamp from path history/{ts}/... -> ts
            let ts = object_path
                .strip_prefix("history/")
                .and_then(|s| s.split('/').next())
                .unwrap_or("")
                .to_string();
            snaps.push((ts, bytes));
        }
    }

    // Sort by timestamp and downsample if needed
    snaps.sort_by(|a, b| a.0.cmp(&b.0));
    let snaps = downsample(&snaps, 100);

    // Build JSON structure for the client side: [{ts, data: <json>}]
    #[derive(serde::Serialize)]
    struct Snapshot<'a> {
        ts: &'a str,
        data: serde_json::Value,
    }
    let mut series: Vec<Snapshot> = Vec::with_capacity(snaps.len());
    for (ts, bytes) in &snaps {
        let text = String::from_utf8_lossy(bytes);
        let data =
            serde_json::from_str::<serde_json::Value>(&text).unwrap_or(serde_json::Value::Null);
        series.push(Snapshot { ts, data });
    }
    let series_js = serde_json::to_string(&series)?;

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
const snapshots = {series};
const problem = "{problem}";

// Labels (timestamps) as Date objects
function parseTs(ts) {{
  const y = +ts.slice(0,4), mo = +ts.slice(4,6)-1, d = +ts.slice(6,8);
  const h = +ts.slice(9,11), mi = +ts.slice(11,13), s = +ts.slice(13,15);
  // Interpret original timestamp as UTC, then Chart.js formats in Asia/Tokyo
  return new Date(Date.UTC(y, mo, d, h, mi, s));
}}
const labels = snapshots.map(s => parseTs(s.ts));

// Build datasets per team (null when no value at a label)
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

function colorFor(name) {{
  let h=0; for (let i=0;i<name.length;i++) h=(h*31+name.charCodeAt(i))>>>0;
  const hue=h%360; return `hsl(${{hue}} 70% 45%)`;
}}

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
      legend: {{ display: false }},
    }},
    scales: {{
      x: {{ type: 'time', time: {{ unit: 'minute' }} }},
      y: {{ beginAtZero: true }},
    }},
    adapters: {{
      date: {{ zone: 'Asia/Tokyo' }},
    }},
  }},
}});

// Build latest-score table
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
if (problem === 'global') {{
  latest.sort((a,b) => b.score - a.score);
}} else {{
  latest.sort((a,b) => a.score - b.score);
}}
// Compute rows with tie-aware ranks and clickable team names
let rows = '';
let lastScore = null;
let lastRank = 0;
latest.forEach((r, i) => {{
  const rank = (lastScore === r.score) ? lastRank : (i + 1);
  lastScore = r.score; lastRank = rank;
  const nameHtml = r.team === 'Unagi' ? `<strong>${{esc(r.team)}}</strong>` : esc(r.team);
  const teamAttr = String(r.team).replace(/[&<>\"']/g, c => ({{'&':'&amp;','<':'&lt;','>':'&gt;','\"':'&quot;','\'':'&#39;'}})[c]);
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
// Click-to-highlight: clicking a team name highlights its series on the chart
let highlightedTeam = null;
function highlightTeam(team) {{
  highlightedTeam = (highlightedTeam === team) ? null : team;
  chart.data.datasets.forEach(ds => {{
    const baseColor = ds.label === 'Unagi' ? '#e53935' : colorFor(ds.label);
    if (highlightedTeam && ds.label !== highlightedTeam) {{
      ds.borderColor = baseColor.startsWith('hsl(')
        ? baseColor.replace('hsl(', 'hsla(').replace(')', ', 0.2)')
        : (baseColor.length === 7 ? baseColor + '33' : baseColor);
      ds.borderWidth = 1;
      ds.pointRadius = 0;
    }} else {{
      ds.borderColor = baseColor;
      ds.borderWidth = (ds.label === 'Unagi') ? 3 : (highlightedTeam ? 3 : 1);
      ds.pointRadius = (ds.label === 'Unagi') ? 3 : (highlightedTeam ? 3 : 1);
    }}
  }});
  chart.update();
}}

document.getElementById('lb-table').addEventListener('click', (ev) => {{
  const a = ev.target.closest('a[data-team]');
  if (!a) return;
  ev.preventDefault();
  const team = a.getAttribute('data-team');
  highlightTeam(team);
}});
</script>
<div style="margin-top:32px;">
  <h3>Latest successful map</h3>
  {map_html}
</div>
"#,
        nav = nav_html,
        problem = problem,
        count = snaps.len(),
        series = series_js,
    );

    Ok(html_page(&format!("Leaderboard - {problem}"), &html))
}
