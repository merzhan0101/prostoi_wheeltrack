use crate::data_base::db_downtimes as db;
// use crate::infra::struct_template as tpw;
use crate::structures::struct_downtimes as strdn;
// use actix_web::{HttpResponse, Responder, get, post, web};
// use askama::Template;

use reqwest::Client;
use serde_json::Value;

use crate::structures::struct_pool_client::AppState;

use crate::areas::find_area;
use crate::infra::struct_template::DowntimesTemplate;
use actix_web::{HttpResponse, Responder, get, post, web};
use askama::Template;

// ________________________________________________________________________________
#[get("/downtimes/{area}")]
async fn render_downtimes_page(
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = find_area(&area_slug) else {
        return HttpResponse::NotFound()
            .content_type("text/plain; charset=utf-8")
            .body("Участок не найден");
    };

    let template = DowntimesTemplate::new(
        area.name,
        area.slug,
        area.department_id,
    );

    match template.render() {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),

        Err(error) => {
            eprintln!("Ошибка отображения страницы: {}", error);

            HttpResponse::InternalServerError()
                .body("Ошибка отображения страницы")
        }
    }
}

//_____________________________
/*#[get("/api/workpiece/downtime/data")]
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

    let mut data_toir = strdn::GetDowntimesRequest::default();
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
}*/
//_____________________________

#[get("/api/{area}/downtimes")]
async fn get_last_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = find_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    let client_http = match Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(client) => client,

        Err(error) => {
            eprintln!(
                "Ошибка создания HTTP-клиента: {}",
                error
            );

            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "error": "Ошибка создания HTTP-клиента"
                }),
            );
        }
    };

    let mut request =
        strdn::GetDowntimesRequest::default();

    request.Filter_DepartmentID =
        Some(area.department_id.to_string());

    request.WithEAMDowntimes = Some(1);
    request.ShowDeleted = Some(0);
    request.MaxRows = Some(50);
    request.Offset = Some(0);

    // println!(
    //     "ЗАПРОС ПРОСТОЕВ: участок={}, DepartmentID={}",
    //     area.name,
    //     area.department_id
    // );

    // println!(
    //     "ФИЛЬТР ТОИР:\n{}",
    //     serde_json::to_string_pretty(&request)
    //         .unwrap_or_default()
    // );

    let post_res = client_http
        .post(format!(
            "{}/get_downtime_list",
            pool.toir_url
        ))
        .basic_auth(
            &pool.toir_login,
            Some(&pool.toir_password),
        )
        .json(&request)
        .send()
        .await;

    match post_res {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!(
                    "ТОИР вернул статус: {}",
                    response.status()
                );

                return HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "Ошибка внешнего сервиса"
                    }),
                );
            }

            // println!
            match response.json::<Value>().await {
                Ok(json) => {
                    // Диагностика ответа ТОИР
                    if let Some(items) = json
                        .get("Data")
                        .and_then(|value| value.as_array())
                    {
                        let mut department_ids: Vec<&str> = items
                            .iter()
                            .filter_map(|item| {
                                item.get("DepartmentID")
                                    .and_then(|value| value.as_str())
                            })
                            .collect();

                        department_ids.sort_unstable();
                        department_ids.dedup();

                        // println!(
                        //     "Участок страницы: {}",
                        //     area.name
                        // );

                        // println!(
                        //     "ТОИР вернул DepartmentID: {:#?}",
                        //     department_ids
                        // );

                        // println!(
                        //     "Backend должен оставить DepartmentID: {}",
                        //     area.department_id
                        // );

                        // println!(
                        //     "Всего записей от ТОИР: {}",
                        //     items.len()
                        // );
                    }

                    match db::get_downtimes_vec_jsonn(
                        &pool,
                        json,
                        area.department_id,
                    )
                    .await
                    {
                        Ok(data) => {
                            // println!(
                            //     "После фильтрации осталось записей: {}",
                            //     data.len()
                            // );

                            HttpResponse::Ok().json(data)
                        }

                        Err(error) => {
                            eprintln!(
                                "Ошибка обработки простоев: {}",
                                error
                            );

                            HttpResponse::InternalServerError().json(
                                serde_json::json!({
                                    "error": "Ошибка обработки данных"
                                }),
                            )
                        }
                    }
                }

                Err(error) => {
                    eprintln!(
                        "Ошибка парсинга JSON ТОИР: {}",
                        error
                    );

                    HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Некорректный ответ ТОИР"
                        }),
                    )
                }
            }

            // BACK
            /*match response.json::<Value>().await {
                Ok(json) => {
                    match db::get_downtimes_vec_jsonn(
                        &pool,
                        json,
                        area.department_id,
                    )
                    .await
                    {
                        Ok(data) => {
                            HttpResponse::Ok().json(data)
                        }

                        Err(error) => {
                            eprintln!(
                                "Ошибка обработки простоев: {}",
                                error
                            );

                            HttpResponse::InternalServerError()
                                .json(
                                    serde_json::json!({
                                        "error": "Ошибка обработки данных"
                                    }),
                                )
                        }
                    }
                }

                Err(error) => {
                    eprintln!(
                        "Ошибка парсинга JSON ТОИР: {}",
                        error
                    );

                    HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Некорректный ответ ТОИР"
                        }),
                    )
                }
            }*/
        }

        Err(error) => {
            eprintln!(
                "Ошибка подключения к ТОИР: {}",
                error
            );

            HttpResponse::BadGateway().json(
                serde_json::json!({
                    "error": "ТОИР не отвечает"
                }),
            )
        }
    }
}

// для сортировки оборудования по буквам
#[get("/api/{area}/equipment")]
async fn get_equipment(
    pool: web::Data<AppState>,
    path: web::Path<String>,
    search: web::Query<strdn::SearchName>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = find_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    match db::get_equipment(
        &pool,
        search.q.clone(),
        area.department_id,
    )
    .await
    {
        Ok(data) => HttpResponse::Ok().json(data),

        Err(error) => {
            eprintln!("Ошибка получения оборудования: {}", error);

            HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "error": "Ошибка получения оборудования"
                }),
            )
        }
    }
}

/*#[get("/api/equipment")]
async fn get_name_depart(
    pool: web::Data<AppState>,
    search: web::Query<strdn::SearchName>,
) -> impl Responder {
    let data = db::get_name_depart(&pool, search.q.clone()).await.unwrap();

    HttpResponse::Ok().json(data)
}*/

// для сортировки рабочих по буквам
#[get("/api/{area}/workers")]
async fn get_name_worker(
    pool: web::Data<AppState>,
    path: web::Path<String>,
    search: web::Query<strdn::SearchName>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = find_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    match db::get_name_worker(
        &pool,
        search.q.clone(),
        area.place_id,
    )
    .await
    {
        Ok(data) => HttpResponse::Ok().json(data),

        Err(error) => {
            eprintln!(
                "Ошибка получения мастеров участка {}: {}",
                area.name,
                error
            );

            HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "error": "Не удалось получить мастеров"
                }),
            )
        }
    }
}

// #[get("/api/workers")]
// async fn get_name_worker(
//     pool: web::Data<AppState>,
//     search: web::Query<strdn::SearchName>,
// ) -> impl Responder {
//     let data = db::get_name_worker(&pool, search.q.clone()).await.unwrap();

//     HttpResponse::Ok().json(data)
// }

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

    let eamid = payload
        .get("EAMID")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let client_http = match Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Ошибка создания HTTP-клиента: {}", error);

            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "Ошибка создания HTTP-клиента"
                }),
            );
        }
    };

    // =========================================================
    // РЕДАКТИРОВАНИЕ СУЩЕСТВУЮЩЕГО ПРОСТОЯ
    // =========================================================
    if !eamid.is_empty() {
        let data_toir_update = match db::data_update(&pool, payload.clone()).await {
            Ok(data) => data,
            Err(error) => {
                eprintln!("Ошибка подготовки данных: {}", error);

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "Ошибка подготовки данных"
                    }),
                );
            }
        };

        let response = match client_http
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&data_toir_update)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                eprintln!("Ошибка запроса в ТОИР: {}", error);

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "ТОИР не отвечает"
                    }),
                );
            }
        };

        let result_http = match response.json::<serde_json::Value>().await {
            Ok(json) => json,
            Err(error) => {
                eprintln!("Ошибка JSON от ТОИР: {}", error);

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "Некорректный ответ ТОИР"
                    }),
                );
            }
        };

        let posted = result_http
            .get("Posted")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);

        if posted != 1 {
            return HttpResponse::Ok().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "ТОИР не принял изменения"
                }),
            );
        }

        if let Err(error) =
            db::update_insert(&pool, payload, result_http).await
        {
            eprintln!("Ошибка обновления локальной БД: {}", error);

            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "Ошибка обновления локальной базы"
                }),
            );
        }

        return HttpResponse::Ok().json(
            serde_json::json!({
                "Posted": 1
            }),
        );
    }

    // =========================================================
    // СОЗДАНИЕ НОВОГО ПРОСТОЯ
    // Здесь оставьте существующую обработку EquipmentIDs
    // =========================================================

    let equipment_ids = payload
        .get("EquipmentIDs")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    if equipment_ids.is_empty() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "Posted": 0,
                "ErrorMessage": "Не выбрано оборудование"
            }),
        );
    }

    // Проверяем, что ни одно выбранное оборудование
    // уже не имеет незавершённого простоя.
    for equipment in &equipment_ids {
        let Some(equipment_id) = equipment.as_str() else {
            continue;
        };

        match db::has_active_downtime(&pool, equipment_id).await {
            Ok(true) => {
                return HttpResponse::Ok().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": format!(
                            "Оборудование уже имеет незавершённый простой. Сначала завершите его через редактирование."
                        )
                    }),
                );
            }

            Ok(false) => {}

            Err(error) => {
                eprintln!(
                    "Ошибка проверки активного простоя оборудования {}: {}",
                    equipment_id,
                    error
                );

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "Ошибка проверки оборудования"
                    }),
                );
            }
        }
    }

    /*let department_ids = payload
        .get("DepartmentIDs")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();*/

    /*let first_department_id = department_ids
        .first()
        .and_then(|value| value.as_str())
        .unwrap_or("");*/

    let current_department_id = payload
        .get("DepartmentID")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if current_department_id.is_empty() {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "Posted": 0,
                "ErrorMessage": "Не передан DepartmentID"
            }),
        );
    }

    for equipment in equipment_ids {
        let Some(equipment_id) = equipment.as_str() else {
            continue;
        };

        let mut one_payload = payload.clone();

        one_payload["EquipmentID"] =
            serde_json::json!(equipment_id);

        one_payload["DepartmentID"] =
            serde_json::json!(current_department_id);
            
        // one_payload["DepartmentID"] =
        //     serde_json::json!(first_department_id);

        if let Some(object) = one_payload.as_object_mut() {
            object.remove("EquipmentIDs");
        }

        let response = match client_http
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&one_payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                eprintln!("Ошибка запроса в ТОИР: {}", error);

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "ТОИР не отвечает"
                    }),
                );
            }
        };

        let result_http = match response.json::<serde_json::Value>().await {
            Ok(json) => json,
            Err(error) => {
                eprintln!("Ошибка JSON от ТОИР: {}", error);

                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "Некорректный ответ ТОИР"
                    }),
                );
            }
        };

        let posted = result_http
            .get("Posted")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);

        if posted != 1 {
            return HttpResponse::Ok().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "ТОИР не принял простой"
                }),
            );
        }

        if let Err(error) =
            db::update_insert(&pool, one_payload, result_http).await
        {
            eprintln!("Ошибка локального сохранения: {}", error);

            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "Ошибка локального сохранения"
                }),
            );
        }
    }

    HttpResponse::Ok().json(
        serde_json::json!({
            "Posted": 1
        }),
    )
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

#[post("/api/{area}/downtimes/filter")]
async fn filter_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<strdn::GetDowntimesRequest>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = find_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    let mut filter = body.into_inner();

    // Участок определяет backend, а не браузер.
    filter.Filter_DepartmentID =
        Some(area.department_id.to_string());

    filter.WithEAMDowntimes = Some(1);
    filter.ShowDeleted = Some(0);
    filter.MaxRows = Some(50);

    // Если Offset не пришёл из браузера.
    if filter.Offset.is_none() {
        filter.Offset = Some(0);
    }

    let client_http = match Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(client) => client,

        Err(error) => {
            eprintln!(
                "Ошибка создания HTTP-клиента: {}",
                error
            );

            return HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "error": "Ошибка создания HTTP-клиента"
                }),
            );
        }
    };

    let post_res = client_http
        .post(format!(
            "{}/get_downtime_list",
            pool.toir_url
        ))
        .basic_auth(
            &pool.toir_login,
            Some(&pool.toir_password),
        )
        .json(&filter)
        .send()
        .await;

    match post_res {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!(
                    "ТОИР вернул статус: {}",
                    response.status()
                );

                return HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "Ошибка внешнего сервиса"
                    }),
                );
            }

            match response.json::<Value>().await {
                Ok(json) => {
                    match db::get_downtimes_vec_jsonn(
                        &pool,
                        json,
                        area.department_id,
                    )
                    .await
                    {
                        Ok(data) => {
                            HttpResponse::Ok().json(data)
                        }

                        Err(error) => {
                            eprintln!(
                                "Ошибка обработки простоев: {}",
                                error
                            );

                            HttpResponse::InternalServerError()
                                .json(
                                    serde_json::json!({
                                        "error": "Ошибка обработки данных"
                                    }),
                                )
                        }
                    }
                }

                Err(error) => {
                    eprintln!(
                        "Ошибка парсинга JSON ТОИР: {}",
                        error
                    );

                    HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Некорректный ответ ТОИР"
                        }),
                    )
                }
            }
        }

        Err(error) => {
            eprintln!(
                "Ошибка подключения к ТОИР: {}",
                error
            );

            HttpResponse::BadGateway().json(
                serde_json::json!({
                    "error": "ТОИР не отвечает"
                }),
            )
        }
    }
}

// для сортировки оборудования по буквам
#[get("/api/departments")]
async fn get_departments(
    pool: web::Data<AppState>,
    search: web::Query<strdn::SearchName>,
) -> impl Responder {
    let data = db::get_name_department(&pool, search.q.clone()).await.unwrap();

    HttpResponse::Ok().json(data)
}

pub fn conf_downtimes(cfg: &mut web::ServiceConfig) {
    cfg.service(render_downtimes_page);
    cfg.service(get_last_downtimes);
    cfg.service(filter_downtimes);
    cfg.service(get_equipment);
    cfg.service(get_name_worker);
    cfg.service(get_downtime_types);
    cfg.service(get_departments);
    cfg.service(update_downtime);
    cfg.service(get_downtime);
}