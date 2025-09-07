//! # API Logging Proxy Handlers
//!
//! This module implements a logging proxy for the official contest API.
//! It defines handlers for the `/select`, `/explore`, and `/guess` endpoints.
//!
//! When a request is received, it is:
//! 1. Forwarded to the official contest server.
//! 2. The original request and the response from the server are both logged to a
//!    database table (`api_logs`) for debugging, analysis, and replay purposes.
//! 3. The response from the official server is then returned to the original caller,
//!    with an additional `X-Unagi-Log` header containing the log ID.

use crate::client;
use crate::sql;

use actix_web::{HttpRequest, HttpResponse, Responder, http::header, web};
use chrono::Utc;
use mysql::params;

use reqwest::header as reqwest_header;
use std::time::Instant;

/// The base URL of the official ICFP 2025 contest server.
const BACKEND_BASE: &str = "https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com";

/// A helper function to remove the `/api` prefix from a request path.
fn strip_api_prefix(path: &str) -> &str {
    if let Some(rest) = path.strip_prefix("/api") {
        if rest.is_empty() { "/" } else { rest }
    } else {
        path
    }
}

/// Forwards a request to the backend server and logs the entire transaction.
///
/// This is the core logic of the proxy. It performs the request forwarding,
/// measures the duration, logs all relevant data to the `api_logs` table,
/// and constructs a response that mirrors the backend's response.
async fn forward_and_log(path: &str, body: web::Bytes, req: &HttpRequest) -> HttpResponse {
    let started = Instant::now();
    let client = &*client::CLIENT;
    let backend_url = format!("{}{}", BACKEND_BASE, path);

    // Forward the request to the official backend and capture the response.
    let (status_code, ct_from_backend, resp_body) = match client
        .post(&backend_url)
        .header(reqwest_header::CONTENT_TYPE, "application/json")
        .body(body.clone())
        .send()
        .await
    {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let ct_from_backend = resp
                .headers()
                .get(reqwest_header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let resp_body = match resp.text().await {
                Ok(t) => t,
                Err(e) => format!("{{\"error\":\"failed to read backend body: {}\"}}", e),
            };
            (status_code, ct_from_backend, resp_body)
        }
        Err(e) => (
            502,
            Some("application/json".to_string()),
            format!("{{\"error\":\"failed to contact backend: {}\"}}", e),
        ),
    };

    // Link logs together in a session, starting from a `/select` call.
    let path_for_log = strip_api_prefix(path);
    let select_id: i64 = if path_for_log == "/select" {
        0
    } else {
        // Find the most recent `/select` call to associate this log with it.
        sql::cell::<i64>(
            "SELECT MAX(api_log_id) FROM api_logs WHERE api_log_path = '/select'",
            (),
        )
        .ok()
        .flatten()
        .unwrap_or(0)
    };

    // Log the transaction to the database.
    let duration_ms = started.elapsed().as_millis() as u64;
    let meta = serde_json::json!({
        "method": req.method().as_str(),
        "path": path_for_log,
        "time": Utc::now().to_rfc3339(),
        "duration_ms": duration_ms,
    })
    .to_string();

    let req_body = String::from_utf8(body.to_vec()).unwrap_or_default();
    let log_id: u64 = sql::insert(
        "INSERT INTO api_logs (api_log_select_id, api_log_path, api_log_metadata, api_log_request, api_log_response_code, api_log_response) VALUES (:sid, :path, :meta, :req, :code, :resp)",
        params! {
            "sid" => select_id,
            "path" => path_for_log,
            "meta" => meta,
            "req" => req_body,
            "code" => status_code as i32,
            "resp" => &resp_body,
        },
    )
    .unwrap_or_default();

    // Build and return a response that mirrors the backend's response,
    // adding a custom header with the log ID.
    let mut builder = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status_code)
            .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY),
    );
    if let Some(ct) = ct_from_backend {
        builder.insert_header((header::CONTENT_TYPE, ct));
    } else {
        builder.insert_header((header::CONTENT_TYPE, "application/json"));
    }
    let header_value = serde_json::json!({
        "api_log_id": log_id,
        "api_duration_ms": duration_ms,
    })
    .to_string();
    builder.insert_header(("X-Unagi-Log", header_value));
    builder.body(resp_body)
}

/// Handles proxying requests to `/select`.
pub async fn post_select(req: HttpRequest, body: web::Bytes) -> impl Responder {
    forward_and_log("/select", body, &req).await
}

/// Handles proxying requests to `/explore`.
pub async fn post_explore(req: HttpRequest, body: web::Bytes) -> impl Responder {
    forward_and_log("/explore", body, &req).await
}

/// Handles proxying requests to `/guess`.
pub async fn post_guess(req: HttpRequest, body: web::Bytes) -> impl Responder {
    forward_and_log("/guess", body, &req).await
}
