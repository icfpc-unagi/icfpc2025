use crate::*;
use actix_web::dev::Service;
use actix_web::http::header::{HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use actix_web::{get, post, web, App, Error, HttpResponse, HttpServer};
use fs2::FileExt;
use futures::FutureExt;
use handlebars::Handlebars;
use itertools::Itertools;
use std::collections::HashSet;
use std::fs::File;
use std::time::Duration;

#[derive(Clone)]
struct MyData {
    settings: Settings,
    handlebars: Handlebars<'static>,
}

impl MyData {
    fn new(settings: Settings) -> Self {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("result", include_str!("../templates/result.html"))
            .expect("Failed to register result");
        handlebars
            .register_template_string("list", include_str!("../templates/list.html"))
            .expect("Failed to register list");
        Self { settings, handlebars }
    }
}

fn get_keys(result: &Vec<ResultData>) -> Vec<String> {
    let mut keys = vec![
        "score", "score_max", "score_min", "score_rank_max", "score_rank_min", "time", "status",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect_vec();
    let mut key_set = keys.iter().cloned().collect::<HashSet<_>>();
    for r in result {
        for log in &r.logs {
            for key in log.keys() {
                if !key_set.contains(key) {
                    key_set.insert(key.clone());
                    keys.push(key.clone());
                }
            }
        }
    }
    keys
}

#[derive(Deserialize, Clone, Debug)]
struct ResultQuery {
    set: Option<String>,
    key: Option<String>,
}

#[get("/")]
async fn index(app_data: web::Data<MyData>, query: web::Query<ResultQuery>) -> Result<HttpResponse, Error> {
    let set_name = query.set.as_ref().unwrap_or(&app_data.settings.default_set);
    let set = Path::new(&set_name);
    let (inputs, input_keys) = get_inputs(set);
    let result = get_result(set, &inputs);
    let keys = get_keys(&result);
    let context = serde_json::json!({
        "set_name": set_name,
        "inputs": serde_json::to_string(&inputs).unwrap(),
        "input_keys": input_keys,
        "result": serde_json::to_string(&result).unwrap(),
        "result_keys": keys,
        "default_key": query.key.clone().unwrap_or(app_data.settings.default_key.clone()),
        "show_details": inputs.len() <= app_data.settings.max_cases
    });
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(app_data.handlebars.render("result", &context).unwrap()))
}

#[get("list")]
async fn list_vis(app_data: web::Data<MyData>, query: web::Query<ResultQuery>) -> Result<HttpResponse, Error> {
    let set_name = query.set.as_ref().unwrap_or(&app_data.settings.default_set);
    let set = Path::new(&set_name);
    let (inputs, input_keys) = get_inputs(set);
    let result = get_result(set, &inputs);
    let keys = get_keys(&result);
    let context = serde_json::json!({
        "set_name": set_name,
        "inputs": serde_json::to_string(&inputs).unwrap(),
        "input_keys": input_keys,
        "result": serde_json::to_string(&result).unwrap(),
        "result_keys": keys,
        "default_key": query.key.clone().unwrap_or(app_data.settings.default_key.clone()),
        "vis_ext": app_data.settings.vis_ext.clone(),
    });
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(app_data.handlebars.render("list", &context).unwrap()))
}

#[get("/vis/recent")]
async fn recent() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../templates/recent.html")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UpdateMsgData {
    set: String,
    id: usize,
    msg: String,
}

#[post("/update_msg")]
async fn update_msg(data: web::Json<UpdateMsgData>) -> Result<HttpResponse, Error> {
    let result = Path::new(&data.set).join("run").join(data.id.to_string()).join("result.toml");
    let msg = data.msg.clone();
    let _ = tokio::task::spawn(async move {
        let s = {
            let mut file = File::open(&result).unwrap();
            while file.try_lock_exclusive().is_err() {
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            let mut s = String::new();
            file.read_to_string(&mut s).unwrap();
            s
        };
        let mut res: RunResult = toml::from_str(&s).unwrap();
        if res.msg != msg {
            res.msg = msg;
            std::fs::write(&result, toml::to_string(&res).unwrap()).unwrap();
        }
    });
    Ok(HttpResponse::Ok().finish())
}

fn configure_files(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .wrap_fn(|req, srv| {
                srv.call(req).map(|res| {
                    res.map(|mut res| {
                        let headers = res.headers_mut();
                        if let Some(ct) = headers.get(CONTENT_TYPE) {
                            if ct.to_str().unwrap_or_default().starts_with("text") {
                                let new_ct = HeaderValue::from_str("text/plain; charset=UTF-8").unwrap();
                                headers.insert(CONTENT_TYPE, new_ct);
                            }
                        }
                        headers.insert(CACHE_CONTROL, HeaderValue::from_str("no-store").unwrap());
                        res
                    })
                })
            })
            .service(actix_files::Files::new("/static", ".")),
    );
}

#[actix_web::main]
pub async fn start(settings: &Settings) -> std::io::Result<()> {
    let data = web::Data::new(MyData::new(settings.clone()));
    let vis = settings.vis.clone();
    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::DefaultHeaders::new().add(("charset", "utf-8")))
            .app_data(data.clone())
            .service(index)
            .service(list_vis)
            .service(recent)
            .service(update_msg)
            .service(actix_files::Files::new("/vis", vis.clone()))
            .configure(configure_files)
    })
    .bind(&settings.listen)?
    .run()
    .await
}
