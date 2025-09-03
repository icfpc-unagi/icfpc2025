use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde_json::json;

static ENGINE: Lazy<Handlebars> = Lazy::new(new_engine);

pub fn new_engine() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(
            "main",
            r#"
<html lang="ja">
<header>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0,user-scalable=yes">
<link rel="stylesheet" type="text/css" href="/static/style.css">
<script src="https://ajax.googleapis.com/ajax/libs/jquery/3.3.1/jquery.min.js"></script>
<script src="/static/jquery-linedtextarea.js"></script>
<link href="/static/jquery-linedtextarea.css" rel="stylesheet"/>
</header>
<body>
<nav>
<a href="/"></a>
<ul>
<li><a href="/my_userboard">問題一覧</a></li>
<li><a href="/my_submissions">提出一覧</a></li>
<li><a href="/visualizer">可視化</a></li>
<li><a href="/comm?q=get+index">index</a> <a href="/comm?q=get+scoreboard">📊</a></li>
<li><a href="/comm?q=get+lambdaman">lambdaman</a> <a href="/comm?q=get+scoreboard+lambdaman">📊</a></li>
<li><a href="/comm?q=get+spaceship">spaceship</a> <a href="/comm?q=get+scoreboard+spaceship">📊</a></li>
<li><a href="/comm?q=get+3d">3d</a> <a href="/comm?q=get+scoreboard+3d">📊</a> <a href="/3d/web/www/index.html">⏯</a></li>
<li><a href="/comm?q=get+efficiency">efficiency</a> <a href="/comm?q=get+scoreboard+efficiency">📊</a></li>
</ul>
</nav>
<main>
<article>
{{{contents}}}
</article>
</main>
</body>
</html>"#,
        )
        .unwrap();
    handlebars
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

pub fn render(contents: &str) -> String {
    ENGINE
        .render(
            "main",
            &json!({
                "contents": contents,
            }),
        )
        .unwrap()
}

pub fn to_error_response(result: &anyhow::Error) -> HttpResponse {
    HttpResponse::InternalServerError()
        .content_type("text/html")
        .body(render(&format!(
            "<h1>エラー</h1><pre><code>{}</code></pre>",
            escape_html(&format!("{:?}", result))
        )))
}

pub fn to_html_response(result: &str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(render(result))
}

pub fn to_png_response(result: &[u8]) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/png")
        .append_header(("Cache-Control", "public, max-age=600"))
        .body(result.to_owned())
}

pub fn to_response(result: Result<String>) -> impl Responder {
    match result {
        Ok(x) => to_html_response(&x),
        Err(e) => to_error_response(&e),
    }
}
