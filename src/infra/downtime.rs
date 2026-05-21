use crate::data_base::workpiece_downtime_db as db;
use crate::infra::struct_templat as tpw;
use crate::structures::workpiece_struct as wsr;
use actix_web::{HttpResponse, Responder, get, post, web};
use askama::Template;

use reqwest::Client;
use serde_json::Value;

use crate::structures::struct_pool_client::AppState;

// ________________________________________________________________________________
#[get("/workpiece/downtimes")]
async fn render_page_downtimes() -> impl Responder {
    match tpw::WorkpieceDowntimesTemplates::render_page().await {
        Ok(template) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(
                template
                    .render()
                    .unwrap_or_else(|_| "Error rendering template".to_string()),
            ),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Error creating template: {}", e))
        }
    }
}

//_____________________________
#[get("/api/workpiece/downtime/data")]
async fn get_last_50row(pool: web::Data<AppState>) -> impl Responder {
    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut data_toir = wsr::GetDowntimesRequest::default();
    data_toir.MaxRows = Some(50);

    let post_res = client_http
        .post("https://pbd02.railsystems.kz/toir_pmk_test_a_maslo/hs/downtime/v1/get_downtime_list")
        .basic_auth("Администратор", Some("qwd123qwd"))
        .json(&data_toir)
        .send()
        .await;

    match post_res {
        Ok(res) => {
            if !res.status().is_success() {
                return HttpResponse::InternalServerError().body("External Service Error");
            }
            match res.json::<Value>().await {
                Ok(json) => {
                    let data = db::get_downtimes_vec_jsonn(&pool, json).await.unwrap();
                    HttpResponse::Ok().json(data)
                }
                Err(e) => {
                    eprintln!("Ошибка парсинга JSON: {}", e);
                    HttpResponse::InternalServerError().body("Invalid JSON from upstream")
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка запроса (сеть): {}", e);
            HttpResponse::InternalServerError().body("Network Error")
        }
    }
}
//_____________________________

// для сортировки оборудования по буквам
#[get("/api/equipment")]
async fn get_name_depart(
    pool: web::Data<AppState>,
    search: web::Query<wsr::SearchName>,
) -> impl Responder {
    let data = db::get_name_depart(&pool, search.q.clone()).await.unwrap();
    HttpResponse::Ok().json(data)
}

// для сортировки рабочих по буквам
#[get("/api/workers")]
async fn get_name_worker(
    pool: web::Data<AppState>,
    search: web::Query<wsr::SearchName>,
) -> impl Responder {
    let data = db::get_name_worker(&pool, search.q.clone()).await.unwrap();
    HttpResponse::Ok().json(data)
}

#[get("/api/downtime-types")]
async fn get_downtime_types(pool: web::Data<AppState>) -> impl Responder {
    let data = db::get_types(&pool).await.unwrap();
    HttpResponse::Ok().json(data)
}

// /GS_downtimeExchange.Method.update_downtime
#[post("/api/update_downtime")]
async fn update_downtime(
    pool: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    // println!("Полученный JSON: {:#?}", body);

    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let payload = body;
    let eamid = payload.get("EAMID").and_then(|v| v.as_str()).unwrap_or("");
    if eamid != "" {
        let data_toir_update = db::data_update(&pool, payload.clone()).await.unwrap();

        let post_res = client_http
            .post(
                "https://pbd02.railsystems.kz/toir_pmk_test_a_maslo/hs/downtime/v1/update_downtime",
            )
            .basic_auth("Администратор", Some("qwd123qwd"))
            .json(&data_toir_update)
            .send()
            .await;

        match post_res {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    // println!("{:#?}", json);
                    let mut data = wsr::GetPosted::default();
                    data.Posted = json
                        .get("Posted")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32);

                    let _ = db::update_insert(&pool, payload.into_inner(), json.clone()).await;
                    HttpResponse::Ok().json(data)
                }
                Err(e) => {
                    let mut data = wsr::GetPosted::default();

                    data.Posted = Some(0);
                    eprintln!("Не удалось распарсить JSON: {}", e);
                    HttpResponse::Ok().json(data)
                }
            },
            Err(e) => {
                let mut data = wsr::GetPosted::default();

                data.Posted = Some(0);
                eprintln!("Ошибка запроса: {}", e);
                HttpResponse::Ok().json(data)
            }
        }
    } else {
        let post_res = client_http
            .post(
                "https://pbd02.railsystems.kz/toir_pmk_test_a_maslo/hs/downtime/v1/update_downtime",
            )
            .basic_auth("Администратор", Some("qwd123qwd"))
            .json(&payload)
            .send()
            .await;

        match post_res {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    // println!("{:#?}", json);
                    let mut data = wsr::GetPosted::default();
                    data.Posted = json
                        .get("Posted")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32);

                    let _ = db::update_insert(&pool, payload.into_inner(), json.clone()).await;
                    HttpResponse::Ok().json(data)
                }
                Err(e) => {
                    let mut data = wsr::GetPosted::default();

                    data.Posted = Some(0);
                    eprintln!("Не удалось распарсить JSON: {}", e);
                    HttpResponse::Ok().json(data)
                }
            },
            Err(e) => {
                let mut data = wsr::GetPosted::default();

                data.Posted = Some(0);
                eprintln!("Ошибка запроса: {}", e);
                HttpResponse::Ok().json(data)
            }
        }
    }
}

#[post("/api/get_downtime")]
async fn get_downtime(
    pool: web::Data<AppState>,
    data_toir: web::Json<serde_json::Value>,
) -> impl Responder {
    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let payload = data_toir;

    // println!("payload: {:#?}", payload);

    let post_res = client_http
        .post("https://pbd02.railsystems.kz/toir_pmk_test_a_maslo/hs/downtime/v1/get_downtime")
        .basic_auth("Администратор", Some("qwd123qwd"))
        .json(&payload)
        .send()
        .await;

    match post_res {
        Ok(res) => {
            // 1. Проверяем статус ответа от внешнего сервиса
            if !res.status().is_success() {
                return HttpResponse::InternalServerError().body("External Service Error");
            }
            // 2. Пытаемся распарсить JSON
            match res.json::<Value>().await {
                Ok(json) => {
                    // println!("{:#?}", json);

                    let data =
                        db::get_data_downtime(&pool, json, &payload["EAMID"].as_str().unwrap())
                            .await
                            .unwrap();

                    HttpResponse::Ok().json(data)
                }
                Err(e) => {
                    eprintln!("Ошибка парсинга JSON: {}", e);
                    HttpResponse::InternalServerError().body("Invalid JSON from upstream")
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка запроса (сеть): {}", e);
            HttpResponse::InternalServerError().body("Network Error")
        }
    }
}

#[post("/api/get_downtime_list/filter")]
async fn get_filter_downtime(
    pool: web::Data<AppState>,
    data_toir: web::Json<wsr::GetDowntimesRequest>,
) -> impl Responder {
    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let post_res = client_http
        .post("https://pbd02.railsystems.kz/toir_pmk_test_a_maslo/hs/downtime/v1/get_downtime_list")
        .basic_auth("Администратор", Some("qwd123qwd"))
        .json(&data_toir)
        .send()
        .await;

    match post_res {
        Ok(res) => {
            if !res.status().is_success() {
                return HttpResponse::InternalServerError().body("External Service Error");
            }
            match res.json::<Value>().await {
                Ok(json) => {
                    // println!("{:#?}", json);
                    let data = db::get_downtimes_vec_jsonn(&pool, json).await.unwrap();
                    HttpResponse::Ok().json(data)
                }
                Err(e) => {
                    eprintln!("Ошибка парсинга JSON: {}", e);
                    HttpResponse::InternalServerError().body("Invalid JSON from upstream")
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка запроса (сеть): {}", e);
            HttpResponse::InternalServerError().body("Network Error")
        }
    }
}

pub fn conf_workpiece_downtime(cfg: &mut web::ServiceConfig) {
    // ========================================================
    cfg.service(render_page_downtimes);
    cfg.service(get_last_50row);
    cfg.service(get_name_depart);
    cfg.service(get_name_worker);
    cfg.service(get_downtime_types);
    cfg.service(update_downtime);
    cfg.service(get_downtime);
    cfg.service(get_filter_downtime);
}
