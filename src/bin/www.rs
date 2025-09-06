use actix_files::Files;
use actix_web::{App, HttpServer, web};
use icfpc2025::www;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let server_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| String::from("0.0.0.0"));
    let server_port = env::var("PORT").unwrap_or_else(|_| String::from("8080"));
    let bind_address = format!("{}:{}", server_address, server_port);

    eprintln!(
        "Starting server at: http://{}/leaderboard/global",
        bind_address
    );
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(www::handlers::index))
            // .route("/comm", web::get().to(www::handlers::comm))
            .route("/cron", web::get().to(www::handlers::cron::run))
            .route(
                "/leaderboard",
                web::get().to(www::handlers::leaderboard::index),
            )
            .route(
                "/leaderboard/{problem}",
                web::get().to(www::handlers::leaderboard::show),
            )
            .route(
                "/api/select",
                web::post().to(www::handlers::api::post_select),
            )
            .route(
                "/api/explore",
                web::post().to(www::handlers::api::post_explore),
            )
            .route("/api/guess", web::post().to(www::handlers::api::post_guess))
            .service(Files::new("/", "/www"))
    })
    .bind(bind_address)?
    .run()
    .await
}
