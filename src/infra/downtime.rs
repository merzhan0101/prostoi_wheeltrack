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
    // let state = pool.pool_raport.state();
    // println!(
    //     "СТАТУС ПУЛА: Всего соединений: {}, Открыто прямо сейчас: {}, {:?}",
    //     state.connections,      // сколько физически открыто в БД
    //     state.idle_connections, // сколько из них СВОБОДНО прямо сейчас   // сколько потоков СЕЙЧАС ждут в очереди (если > 0, мы в дедлоке)
    //     state.statistics.get_wait_time
    // );

    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut data_toir = wsr::GetDowntimesRequest::default();
    data_toir.MaxRows = Some(50);

    let post_res = client_http
        .post(format!("{}/get_downtime_list", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
        .json(&data_toir)
        .send()
        .await;

    // println!("Отправляем запрос на {} с данными: {:#?}", format!("{}/get_downtime_list", pool.toir_url), data_toir);

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
    let payload = body.into_inner();

    let equipment_ids = payload
        .get("EquipmentIDs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if equipment_ids.is_empty() {
        let mut data = wsr::GetPosted::default();
        data.Posted = Some(0);
        return HttpResponse::Ok().json(data);
    }

    // Получаем все выбранные службы
    let department_ids = payload
        .get("DepartmentIDs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Первая служба для записи в локальную БД
    let first_department_id = department_ids
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let client_http = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    for eq in equipment_ids {
        let Some(eq_id) = eq.as_str() else {
            continue;
        };

        let mut one_payload = payload.clone();
        one_payload["EquipmentID"] = serde_json::json!(eq_id);

        // Для локальной БД
        one_payload["DepartmentID"] = serde_json::json!(first_department_id);

        // DepartmentIDs оставляем, чтобы он ушел в ТОИР

        // чтобы в ТОИР не улетал EquipmentIDs
        if let Some(obj) = one_payload.as_object_mut() {
            obj.remove("EquipmentIDs");
        }

        let post_res = client_http
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&one_payload)
            .send()
            .await;

        let json = match post_res {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Не удалось распарсить JSON от ТОИР: {}", e);

                    let mut data = wsr::GetPosted::default();
                    data.Posted = Some(0);
                    return HttpResponse::Ok().json(data);
                }
            },
            Err(e) => {
                eprintln!("Ошибка запроса в ТОИР: {}", e);

                let mut data = wsr::GetPosted::default();
                data.Posted = Some(0);
                return HttpResponse::Ok().json(data);
            }
        };

        let posted = json
            .get("Posted")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if posted != 1 {
            eprintln!("ТОИР не принял простой: {:#?}", json);

            let mut data = wsr::GetPosted::default();
            data.Posted = Some(0);
            return HttpResponse::Ok().json(data);
        }

        if let Err(e) = db::update_insert(&pool, one_payload, json).await {
            eprintln!("Ошибка локального сохранения: {}", e);

            let mut data = wsr::GetPosted::default();
            data.Posted = Some(0);
            return HttpResponse::Ok().json(data);
        }
    }

    let mut data = wsr::GetPosted::default();
    data.Posted = Some(1);
    HttpResponse::Ok().json(data)
}


/* back
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
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
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
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
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
}*/

// локально, без отправки в ТОИР
/*#[post("/api/update_downtime")]
async fn update_downtime(
    pool: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let payload = body.into_inner();

    let equipment_ids = payload
        .get("EquipmentIDs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if equipment_ids.is_empty() {
        let mut data = wsr::GetPosted::default();
        data.Posted = Some(0);
        return HttpResponse::Ok().json(data);
    }

    for eq in equipment_ids {
        let Some(eq_id) = eq.as_str() else {
            continue;
        };

        let mut one_payload = payload.clone();
        one_payload["EquipmentID"] = serde_json::json!(eq_id);

        let fake_result = serde_json::json!({
            "Posted": 1,
            "EAMID": uuid::Uuid::new_v4().to_string(),
            "EAMDocName": "",
            "EAMVersion": ""
        });

        if let Err(e) = db::update_insert(&pool, one_payload, fake_result).await {
            eprintln!("Ошибка локального сохранения: {}", e);

            let mut data = wsr::GetPosted::default();
            data.Posted = Some(0);
            return HttpResponse::Ok().json(data);
        }
    }

    let mut data = wsr::GetPosted::default();
    data.Posted = Some(1);
    HttpResponse::Ok().json(data)
}*/

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
        .post(format!("{}/get_downtime", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
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
                    // println!("GET_DOWNTIME JSON: {:#?}", json);

                    let data =
                        db::get_data_downtime(&pool, json, &payload["EAMID"].as_str().unwrap())
                            .await
                            .unwrap();

                    // println!("DATA TO FRONT: {:#?}", data);

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
        .post(format!("{}/get_downtime_list", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
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

// test
// для сортировки оборудования по буквам
#[get("/api/departments")]
async fn get_departments(
    pool: web::Data<AppState>,
    search: web::Query<wsr::SearchName>,
) -> impl Responder {
    let data = db::get_name_department(&pool, search.q.clone()).await.unwrap();

    HttpResponse::Ok().json(data)
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
    cfg.service(get_departments);
}