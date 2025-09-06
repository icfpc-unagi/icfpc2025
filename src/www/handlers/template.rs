//! # HTML Templating and Response Helpers
//!
//! This module provides a simple HTML templating system using the `handlebars`
//! crate. It defines a single main page layout and offers helper functions
//! to render content within this layout. It also includes several utility
//! functions for creating common `actix_web::HttpResponse` objects.

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde_json::json;

/// A lazily-initialized, global instance of the Handlebars templating engine.
static ENGINE: Lazy<Handlebars> = Lazy::new(new_engine);

/// Creates and configures a new `Handlebars` engine instance.
///
/// This function registers a single template string named "main", which serves
/// as the main HTML layout for all pages. The layout includes a common header,
/// navigation, and a `{{{contents}}}` placeholder where page-specific content
/// will be injected.
pub fn new_engine() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string(
            "main",
            r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0,user-scalable=yes">
<link rel="stylesheet" type="text/css" href="/static/style.css">
<script src="https://ajax.googleapis.com/ajax/libs/jquery/3.3.1/jquery.min.js"></script>
<script src="/static/jquery-linedtextarea.js"></script>
<link href="/static/jquery-linedtextarea.css" rel="stylesheet"/>
</head>
<body>
<nav>
<a href="/"></a>
<ul>
<li><a href="/leaderboard/global">リーダーボード</a></li>
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

/// A simple utility to escape HTML special characters.
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

/// Renders the given content string into the main HTML layout.
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

/// Creates an HTML response for displaying an `anyhow::Error`.
///
/// The error is formatted within a `<pre>` block inside the main page layout.
pub fn to_error_response(result: &anyhow::Error) -> HttpResponse {
    HttpResponse::InternalServerError()
        .content_type("text/html")
        .body(render(&format!(
            "<h1>エラー</h1><pre><code>{}</code></pre>",
            escape_html(&format!("{:?}", result))
        )))
}

/// Creates a standard HTML `Ok` response from a string slice.
pub fn to_html_response(result: &str) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(render(result))
}

/// Creates a PNG image response from a byte slice.
///
/// The response is given a `max-age` cache header of 10 minutes.
pub fn to_png_response(result: &[u8]) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/png")
        .append_header(("Cache-Control", "public, max-age=600"))
        .body(result.to_owned())
}

/// A generic helper that converts a `Result<String>` into an appropriate HTML response.
pub fn to_response(result: Result<String>) -> impl Responder {
    match result {
        Ok(x) => to_html_response(&x),
        Err(e) => to_error_response(&e),
    }
}
