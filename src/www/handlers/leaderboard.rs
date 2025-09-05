use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use serde::Deserialize;

fn html_page(title: &str, body: &str) -> String {
    crate::www::handlers::template::render(&format!("<h1>{}</h1>\n{}", title, body))
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
    let mut series_json_parts: Vec<String> = Vec::new();
    for (ts, bytes) in &snaps {
        let text = String::from_utf8_lossy(bytes);
        // As JSON value; if it's invalid, wrap as null to avoid breaking page
        let parsed = match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(v) => v,
            Err(_) => serde_json::Value::Null,
        };
        series_json_parts.push(format!(
            "{{\"ts\":\"{}\",\"data\":{}}}",
            ts,
            serde_json::to_string(&parsed)?
        ));
    }
    let series_js = format!("[{}]", series_json_parts.join(","));

    let html = format!(
        r#"
<div>
  <h2>Problem: {problem}</h2>
  <p>Snapshots: {count}</p>
</div>
<div id="chart" style="width: 100%; height: 600px;"></div>
<script src="https://d3js.org/d3.v7.min.js"></script>
<script>
const snapshots = {series};

// Build team -> time series with real timestamps
const parseTs = d3.timeParse("%Y%m%d-%H%M%S");
const seriesByTeam = new Map();
for (const snap of snapshots) {
  const date = parseTs(snap.ts);
  if (!date) continue;
  const arr = Array.isArray(snap.data) ? snap.data : [];
  for (const rec of arr) {
    const team = rec.teamName;
    const score = rec.score;
    if (!team || score == null) continue;
    if (!seriesByTeam.has(team)) seriesByTeam.set(team, []);
    seriesByTeam.get(team).push({ date, score: +score });
  }
}
for (const pts of seriesByTeam.values()) {
  pts.sort((a,b) => a.date - b.date);
}

// Compute domains
const allPoints = Array.from(seriesByTeam.values()).flat();
if (allPoints.length === 0) {
  document.getElementById('chart').innerText = 'No data';
} else {
  const xExtent = d3.extent(allPoints, d => d.date);
  const yMax = d3.max(allPoints, d => d.score) || 0;

  // Layout
  const container = document.getElementById('chart');
  const width = container.clientWidth;
  const height = container.clientHeight;
  const margin = { top: 20, right: 20, bottom: 40, left: 60 };
  const iw = Math.max(100, width - margin.left - margin.right);
  const ih = Math.max(100, height - margin.top - margin.bottom);

  const svg = d3.select(container)
    .append('svg')
    .attr('width', width)
    .attr('height', height);

  const g = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);

  const x = d3.scaleTime().domain(xExtent).range([0, iw]);
  const y = d3.scaleLinear().domain([0, yMax]).nice().range([ih, 0]);

  g.append('g')
    .attr('transform', `translate(0,${ih})`)
    .call(d3.axisBottom(x));
  g.append('g').call(d3.axisLeft(y));

  const color = d3.scaleOrdinal(d3.schemeCategory10)
    .domain(Array.from(seriesByTeam.keys()));

  const line = d3.line()
    .x(d => x(d.date))
    .y(d => y(d.score));

  for (const [team, pts] of seriesByTeam.entries()) {
    if (pts.length < 2) continue;
    g.append('path')
      .datum(pts)
      .attr('fill', 'none')
      .attr('stroke', color(team))
      .attr('stroke-width', 1.5)
      .attr('d', line);
  }
}
</script>
"#,
        problem = problem,
        count = snaps.len(),
        series = series_js,
    );

    Ok(html_page(&format!("Leaderboard - {}", problem), &html))
}
