//! # Web Request Handlers
//!
//! This module aggregates all the Axum request handlers for the web server.
//! Each submodule corresponds to a specific area of functionality or API endpoint.

use crate::*;
// pub mod api_proxy;
pub mod cron;
// pub mod my_submissions;
// pub mod my_userboard;
// pub mod problem_png;
// pub mod submission;
// pub mod submissions;
pub mod template;
// pub mod visualize;
pub mod api;
pub mod leaderboard;
pub mod task;
pub mod unlock;

use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

/// Handler for the root path (`/`).
///
/// Responds with a simple "Hello, world!" page.
pub async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(www::handlers::template::render(
            "Hello, world!<br><a href='/comm'>communicate</a>",
        ))
}

/// Query parameters for the `/comm` endpoint.
#[derive(Deserialize)]
pub struct CommQuery {
    /// The query string.
    #[serde(default)]
    _q: String,
    /// A flag for raw mode.
    #[serde(default)]
    _raw: bool,
}

/// Handler for the `/comm` endpoint.
///
/// Note: The current implementation of this handler is a stub and
/// its main logic is commented out. It currently returns an empty `Ok` response.
pub async fn comm(_query: web::Query<CommQuery>) -> impl Responder {
    HttpResponse::Ok()
}

// NOTE: The original implementation of the `comm` handler is preserved below
// in a commented-out block for historical reference.
//
// pub async fn comm(query: web::Query<CommQuery>) -> impl Responder {
//     let mut value_str = String::new();
//     let mut response = String::new();
//     if !query.q.is_empty() {
//         response = match communicate_async(if query.raw {
//             query.q.to_owned()
//         } else {
//             "S".to_owned() + &encode_str(&query.q)
//         })
//         .await
//         {
//             Ok(body) => body,
//             Err(error) => return HttpResponse::InternalServerError().body(error.to_string()),
//         };
//         // let value = eval::eval(&response);
//         // let value_str = if let eval::Value::Str(s) = value {
//         //     String::from_utf8(s).unwrap_or_default()
//         // } else {
//         //     format!("{}", value)
//         // };
//         // echo eval
//         value_str = match communicate_async("B. S%#(/} ".to_owned() + &response).await {
//             Ok(value) => {
//                 let decoded = decode_str(value.strip_prefix('S').unwrap_or(&value));
//                 decoded
//                     .strip_suffix("\n\nYou scored some points for using the echo service!\n")
//                     .unwrap_or(&decoded)
//                     .to_owned()
//             }
//             Err(error) => return HttpResponse::InternalServerError().body(error.to_string()),
//         };
//     }
//     HttpResponse::Ok()
//         .content_type("text/html")
//         .body(www::handlers::template::render(&format!(
//             r#"
//             <form>
//                 <textarea name="q" placeholder="message" autofocus required cols="160">{}</textarea>
//                 <div>
//                     <input type="checkbox" name="raw" id="raw" value="true"{}><label for="raw">raw</label>
//                     <button type="submit">Send</button>
//                 </div>
//             </form>
//             <h4>Decoded response:</h4>
//             <textarea placeholder="response" readonly cols="160" rows="20" id="response">{}</textarea>
//             <h4>Rendered response:</h4>
//             <section id="rendered" style="font-size:x-small">
//                 <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
//                 <script>
//                     try {{
//                         let r = document.getElementById("response");
//                         let md = r.value.replace(/\[([a-z0-9-]+)\](?!\()/g, "[$1](?q=get+$1)");
//                         document.write(marked.parse(md));
//                     }} catch (e) {{}}
//                 </script>
//             </section>
//             <h4>Raw response:</h4>
//             <textarea placeholder="raw response" readonly cols="160" rows="10">{}</textarea>
//             "#,
//             query.q,
//             if query.raw { " checked" } else { "" },
//             value_str,
//             response,
//         )))
// }
