use crate::lock;
use crate::sql;
use crate::www::handlers::template;
use actix_web::{Responder, web};
use mysql::params;

/// Handler for /lock endpoint. GET: show active lock user and POST button. POST: forcibly unlock.
pub async fn unlock_get() -> impl Responder {
    // Query active lock user and token
    let row = match sql::row(
        r#"SELECT lock_user, lock_token FROM locks WHERE lock_id = 1 AND lock_expired > CURRENT_TIMESTAMP LIMIT 1"#,
        params::Params::Empty,
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            let contents = "<h2>(no active lock)</h2>".to_string();
            return actix_web::HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(template::render(&contents));
        }
        Err(e) => return template::to_error_response(&e),
    };
    let user = row.at::<String>(0).unwrap_or("(unknown)".to_string());
    let token = row.at::<String>(1).unwrap_or("".to_string());
    let contents = format!(
        r#"<h2>Active lock user: {user} ({token})</h2>
<form method='POST' action='/unlock'>
    <input type='hidden' name='lock_token' value='{token}'>
    <button type='submit'>!!! Unlock 🔓️ !!!</button>
</form>"#,
    );
    actix_web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(template::render(&contents))
}

#[derive(serde::Deserialize)]
pub struct UnlockForm {
    pub lock_token: String,
}

pub async fn unlock_post(form: web::Form<UnlockForm>) -> impl Responder {
    let res = lock::unlock(&form.lock_token, false);
    match res {
        Ok(_) => actix_web::HttpResponse::Found()
            .append_header(("Location", "/leaderboard/global"))
            .finish(),
        Err(e) => template::to_error_response(&e),
    }
}
