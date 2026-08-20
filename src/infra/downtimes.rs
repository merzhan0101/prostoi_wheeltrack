use crate::data_base::db_downtimes as db;
use crate::structures::struct_downtimes as strdn;
use reqwest::Client;
use serde_json::Value;
use crate::structures::struct_pool_client::AppState;
use crate::areas::find_area;
use crate::infra::struct_template::{DowntimesTemplate, MonitoringTemplate};
use actix_web::{HttpResponse, Responder, get, post, web};
use askama::Template;
use uuid::Uuid;
use chrono::Local;

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

// ________________________________________________________________________________
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

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ 
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Загрузка данных из локальной БД");
        
        match db::get_downtimes_local(&pool, area.department_id).await {
            Ok(data) => {
                // println!("✅ Загружено {} записей из локальной БД", data.len());
                HttpResponse::Ok().json(data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки локальных данных: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {
        // =========================================================
        // РЕЖИМ ТОИР
        // =========================================================

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
}

// ________________________________________________________________________________
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

// ________________________________________________________________________________
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

// ________________________________________________________________________________
#[get("/api/downtime-types")]
async fn get_downtime_types(pool: web::Data<AppState>) -> impl Responder {
    let data = db::get_types(&pool).await.unwrap();

    HttpResponse::Ok().json(data)
}
 
// ________________________________________________________________________________
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

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ 
    // =========================================================
    if pool.local_mode {

        let mut payload_with_name = payload.clone();
        
        // Добавляем флаг локального режима
        payload_with_name["_local_mode"] = serde_json::json!(true);
        
        // Получаем DepartmentID из payload
        let dept_id = payload
            .get("DepartmentID")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let department_id = if dept_id.is_empty() {
            "92b138d1-c131-11ed-8163-00155d16fe68"
        } else {
            dept_id
        };
        payload_with_name["DepartmentID"] = serde_json::json!(department_id);
        
        // =========================================================
        // ОБНОВЛЕНИЕ СУЩЕСТВУЮЩЕЙ ЗАПИСИ (EAMID не пустой)
        // =========================================================
        if !eamid.is_empty() {
            // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Обновление записи {}", eamid);
            
            // Получаем EquipmentID
            if let Some(eq_ids) = payload.get("EquipmentIDs").and_then(|v| v.as_array()) {
                if let Some(first_eq) = eq_ids.first() {
                    if let Some(eq_id) = first_eq.as_str() {
                        payload_with_name["EquipmentID"] = serde_json::json!(eq_id);
                    }
                }
            }
            
            // Генерируем фейковый результат для обновления
            let fake_result = serde_json::json!({
                "Posted": 1,
                "EAMID": eamid,
                "EAMDocName": payload.get("EAMDocName").and_then(|v| v.as_str()).unwrap_or(""),
                "EAMVersion": "LOCAL_VERSION",
            });
            
            // Обновляем в локальной БД
            match db::update_insert(&pool, payload_with_name, fake_result.clone()).await {
                Ok(_) => {
                    return HttpResponse::Ok().json(
                        serde_json::json!({
                            "Posted": 1,
                            "Message": "✅ Простой обновлен локально (без ТОИР)",
                            "LocalMode": true,
                            "Data": fake_result
                        })
                    );
                }
                Err(error) => {
                    eprintln!("❌ Ошибка локального обновления: {}", error);
                    return HttpResponse::InternalServerError().json(
                        serde_json::json!({
                            "Posted": 0,
                            "ErrorMessage": format!("❌ Ошибка обновления: {}", error)
                        })
                    );
                }
            }
        }
        
        // =========================================================
        // СОЗДАНИЕ НОВОЙ ЗАПИСИ (EAMID пустой)
        // =========================================================
        // ВАЖНО: Берем EquipmentID из EquipmentIDs
        if let Some(eq_ids) = payload.get("EquipmentIDs").and_then(|v| v.as_array()) {
            if let Some(first_eq) = eq_ids.first() {
                if let Some(eq_id) = first_eq.as_str() {
                    payload_with_name["EquipmentID"] = serde_json::json!(eq_id);
                    
                    // Добавляем имя оборудования для отображения
                    let equipment_name = format!("Оборудование-{}", &eq_id[..8]);
                    payload_with_name["EquipmentName"] = serde_json::json!(equipment_name);
                }
            }
        }
        
        // Генерируем фейковые данные для новой записи
        let fake_result = serde_json::json!({
            "Posted": 1,
            "EAMID": Uuid::new_v4().to_string(),
            "EAMDocName": format!("ЛОКАЛЬНЫЙ-{}", Local::now().format("%Y%m%d%H%M%S")),
            "EAMVersion": "LOCAL_VERSION",
        });
        
        // Сохраняем в локальную БД
        match db::update_insert(&pool, payload_with_name, fake_result.clone()).await {
            Ok(_) => {
                return HttpResponse::Ok().json(
                    serde_json::json!({
                        "Posted": 1,
                        "Message": "✅ Простой сохранен локально (без ТОИР)",
                        "LocalMode": true,
                        "Data": fake_result
                    })
                );
            }
            Err(error) => {
                eprintln!("❌ Ошибка локального сохранения: {}", error);
                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": format!("❌ Ошибка сохранения: {}", error)
                    })
                );
            }
        }
    }
    
    // =========================================================
    // РЕЖИМ ТОИР
    // =========================================================

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
        // Создаем payload для обновления в том же формате, что и для создания
        let mut update_payload = payload.clone();
        
        // Убеждаемся, что EAMID заполнен
        update_payload["EAMID"] = serde_json::json!(eamid);
        
        // Добавляем Area если есть
        if let Some(area) = payload.get("Area").and_then(|v| v.as_str()) {
            if !area.is_empty() {
                update_payload["Area"] = serde_json::json!(area);
            }
        }
        
        // Добавляем DepartmentIDs если есть
        if let Some(dept_ids) = payload.get("DepartmentIDs").and_then(|v| v.as_array()) {
            if !dept_ids.is_empty() {
                update_payload["DepartmentIDs"] = serde_json::json!(dept_ids);
            }
        }
        
        // Удаляем EquipmentIDs если есть (при обновлении только одно оборудование)
        if let Some(object) = update_payload.as_object_mut() {
            object.remove("EquipmentIDs");
        }
        
        // Проверяем, есть ли EquipmentID
        let equipment_id = update_payload
            .get("EquipmentID")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if equipment_id.is_empty() {
            return HttpResponse::BadRequest().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "Не указано оборудование"
                })
            );
        }

        // println!("Отправка в ТОИР (обновление через payload): {:#?}", update_payload);

        let response = match client_http
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&update_payload)
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
                    })
                );
            }
        };

        // Получаем текст ответа
        let response_text = match response.text().await {
            Ok(text) => {
                // println!("Ответ ТОИР (обновление): {}", text);
                text
            }
            Err(error) => {
                eprintln!("Ошибка чтения ответа ТОИР: {}", error);
                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "Posted": 0,
                        "ErrorMessage": "Ошибка чтения ответа ТОИР"
                    })
                );
            }
        };

        // Проверяем, является ли ответ JSON
        if response_text.trim().starts_with('{') {
            match serde_json::from_str::<serde_json::Value>(&response_text) {
                Ok(result_http) => {
                    let posted = result_http
                        .get("Posted")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    if posted != 1 {
                        let error_msg = result_http
                            .get("ErrorMessage")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Неизвестная ошибка");
                        
                        return HttpResponse::Ok().json(
                            serde_json::json!({
                                "Posted": 0,
                                "ErrorMessage": format!("❌ ТОИР не принял изменения: {}", error_msg)
                            })
                        );
                    }

                    // Обновляем локальную БД
                    if let Err(error) = db::update_insert(&pool, update_payload, result_http).await {
                        eprintln!("Ошибка обновления локальной БД: {}", error);
                        return HttpResponse::InternalServerError().json(
                            serde_json::json!({
                                "Posted": 0,
                                "ErrorMessage": "Ошибка обновления локальной базы"
                            })
                        );
                    }

                    // УСПЕХ - возвращаем единообразный ответ
                    return HttpResponse::Ok().json(
                        serde_json::json!({
                            "Posted": 1,
                            "Message": "✅ Простой успешно обновлен"
                        })
                    );
                }
                Err(error) => {
                    eprintln!("Ошибка парсинга JSON от ТОИР: {}", error);
                    eprintln!("Ответ ТОИР: {}", response_text);
                    
                    return HttpResponse::Ok().json(
                        serde_json::json!({
                            "Posted": 0,
                            "ErrorMessage": format!("❌ Некорректный ответ ТОИР: {}", response_text)
                        })
                    );
                }
            }
        } else {
            eprintln!("ТОИР вернул не JSON: {}", response_text);
            
            return HttpResponse::Ok().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": format!("❌ Ошибка в ТОИР: {}", response_text)
                })
            );
        }
    }


    // =========================================================
    // СОЗДАНИЕ НОВОГО ПРОСТОЯ
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

// ________________________________________________________________________________
#[post("/api/get_downtime")]
async fn get_downtime(
    pool: web::Data<AppState>,
    data_toir: web::Json<serde_json::Value>,
) -> impl Responder {
    let payload = data_toir.into_inner();
    let eamid = payload.get("EAMID").and_then(|v| v.as_str()).unwrap_or("");

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Загрузка данных для редактирования из локальной БД");
        
        match db::get_downtime_local(&pool, eamid).await {
            Ok(data) => {
                // println!("✅ Данные загружены из локальной БД");
                HttpResponse::Ok().json(data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки данных: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {
        // =========================================================
        // РЕЖИМ ТОИР
        // =========================================================
        let client_http = match Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                eprintln!("Ошибка создания HTTP-клиента: {}", error);
                return HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка создания HTTP-клиента"
                    })
                );
            }
        };

        let post_res = client_http
            .post(format!("{}/get_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&payload)
            .send()
            .await;

        match post_res {
            Ok(res) => {
                if !res.status().is_success() {
                    return HttpResponse::InternalServerError().body("External Service Error");
                }
                match res.json::<Value>().await {
                    Ok(json) => {
                        // println!("GET_DOWNTIME JSON: {:#?}", json);

                        let data =
                        db::get_data_downtime(&pool, json, &payload["EAMID"].as_str().unwrap())
                            .await
                            .unwrap();

                        // println!("DATA TO FRONT: {:#?}", data);

                        HttpResponse::Ok().json(data)
                        
                        /*match db::get_data_downtime(&pool, json, eamid).await {
                            Ok(data) => HttpResponse::Ok().json(data),
                            Err(e) => {
                                eprintln!("Ошибка обработки данных: {}", e);
                                HttpResponse::InternalServerError().body("Data processing error")
                            }
                        }*/
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
}

// ________________________________________________________________________________
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

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Фильтрация данных из локальной БД");
        
        match db::get_downtimes_local(&pool, area.department_id).await {
            Ok(data) => {
                let filter = body.into_inner();
                
                // Фильтруем по дате если указана
                let filtered_data = if let Some(start_date) = filter.Filter_StartDate {
                    let date_str = &start_date[0..8]; // YYYYMMDD
                    data.into_iter()
                        .filter(|item| {
                            if let Some(dt) = &item.StartDateTime {
                                dt.starts_with(date_str)
                            } else {
                                false
                            }
                        })
                        .collect()
                } else {
                    data
                };
                
                // println!("✅ Загружено {} записей из локальной БД", filtered_data.len());
                HttpResponse::Ok().json(filtered_data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки локальных данных: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {

        // =========================================================
        // РЕЖИМ ТОИР
        // =========================================================

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

async fn get_version_from_toir(
    client: &Client,
    pool: &AppState,
    eamid: &str,
) -> Option<String> {
    let request = serde_json::json!({
        "EAMID": eamid
    });

    // println!("Запрос версии в ТОИР: {:#?}", request);

    match client
        .post(format!("{}/get_downtime", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
        .json(&request)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            // println!("Статус ответа ТОИР (get_downtime): {}", status);
            
            if !status.is_success() {
                eprintln!("ТОИР вернул ошибку: {}", status);
                return None;
            }

            match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    // println!("Ответ ТОИР (get_downtime): {:#?}", json);
                    
                    // Ищем EAMVersion в разных местах
                    if let Some(version) = json
                        .get("Data")
                        .and_then(|d| d.get("EAMVersion"))
                        .and_then(|v| v.as_str())
                    {
                        if !version.is_empty() {
                            // println!("Найдена версия в Data.EAMVersion: {}", version);
                            return Some(version.to_string());
                        }
                    }

                    if let Some(version) = json
                        .get("EAMVersion")
                        .and_then(|v| v.as_str())
                    {
                        if !version.is_empty() {
                            // println!("Найдена версия в EAMVersion: {}", version);
                            return Some(version.to_string());
                        }
                    }

                    // Если EAMVersion нет, это нормально для некоторых документов
                    // println!("EAMVersion не найден в ответе ТОИР, это допустимо");
                    None
                }
                Err(e) => {
                    eprintln!("Ошибка парсинга JSON: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка запроса к ТОИР: {}", e);
            None
        }
    }
}

// МОНИТОРИНГ ПРОСТОИ ==================================================================================
// ________________________________________________________________________________
#[get("/monitoring")]
async fn render_monitoring_page() -> impl Responder {
    let template = MonitoringTemplate::new();

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

// ________________________________________________________________________________
#[get("/api/{area}/monitoring")]
async fn get_area_monitoring(
    pool: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ 
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Загрузка мониторинга для {} из локальной БД", area.name);
        
        match db::get_area_monitoring_local(&pool, area).await {
            Ok(data) => {
                // println!("✅ Загружен мониторинг для {}", area.name);
                HttpResponse::Ok().json(data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки локального мониторинга: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {
        // РЕЖИМ ТОИР
        match db::get_area_monitoring(&pool, area).await {
            Ok(data) => HttpResponse::Ok().json(data),
            Err(error) => {
                eprintln!("Ошибка получения статуса участка: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка получения статуса участка"
                    }),
                )
            }
        }
    }
}


/*#[get("/api/{area}/monitoring")]
async fn get_area_monitoring(
    pool: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    match db::get_area_monitoring(&pool, area).await {
        Ok(data) => HttpResponse::Ok().json(data),

        Err(error) => {
            eprintln!("Ошибка получения статуса участка: {}", error);

            HttpResponse::InternalServerError().json(
                serde_json::json!({
                    "error": "Ошибка получения статуса участка"
                }),
            )
        }
    }
}*/

// ________________________________________________________________________________
#[get("/api/monitoring/{area}/downtimes")]
async fn get_monitoring_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ 
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Загрузка простоев мониторинга для {} из локальной БД", area.name);
        
        match db::get_monitoring_downtimes_local(&pool, area).await {
            Ok(data) => {
                // println!("✅ Загружено {} простоев для {}", data.len(), area.name);
                HttpResponse::Ok().json(data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки локальных простоев: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {
        // ОБЫЧНЫЙ РЕЖИМ - берем данные из ТОИР
        // ... существующий код ...
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

        let mut request = strdn::GetDowntimesRequest::default();

        request.Filter_DepartmentID = Some(area.department_id.to_string());
        request.WithEAMDowntimes = Some(1);
        request.ShowDeleted = Some(0);
        request.MaxRows = Some(50);
        request.Offset = Some(0);

        let post_res = client_http
            .post(format!("{}/get_downtime_list", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&request)
            .send()
            .await;

        match post_res {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("ТОИР вернул статус: {}", response.status());

                    return HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Ошибка внешнего сервиса"
                        }),
                    );
                }

                match response.json::<Value>().await {
                    Ok(json) => {
                        match db::get_downtimes_vec_jsonn_by_equipment(
                            &pool,
                            json,
                            area.equipment_ids,
                        )
                        .await
                        {
                            Ok(data) => HttpResponse::Ok().json(data),

                            Err(error) => {
                                eprintln!("Ошибка обработки простоев: {}", error);

                                HttpResponse::InternalServerError().json(
                                    serde_json::json!({
                                        "error": "Ошибка обработки данных"
                                    }),
                                )
                            }
                        }
                    }

                    Err(error) => {
                        eprintln!("Ошибка парсинга JSON ТОИР: {}", error);

                        HttpResponse::BadGateway().json(
                            serde_json::json!({
                                "error": "Некорректный ответ ТОИР"
                            }),
                        )
                    }
                }
            }

            Err(error) => {
                eprintln!("Ошибка подключения к ТОИР: {}", error);

                HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "ТОИР не отвечает"
                    }),
                )
            }
        }
    }
}

/*#[get("/api/monitoring/{area}/downtimes")]
async fn get_monitoring_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
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

    let mut request = strdn::GetDowntimesRequest::default();

    request.Filter_DepartmentID = Some(area.department_id.to_string());
    request.WithEAMDowntimes = Some(1);
    request.ShowDeleted = Some(0);
    request.MaxRows = Some(50);
    request.Offset = Some(0);

    let post_res = client_http
        .post(format!("{}/get_downtime_list", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
        .json(&request)
        .send()
        .await;

    match post_res {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("ТОИР вернул статус: {}", response.status());

                return HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "Ошибка внешнего сервиса"
                    }),
                );
            }

            match response.json::<Value>().await {
                Ok(json) => {
                    match db::get_downtimes_vec_jsonn_by_equipment(
                        &pool,
                        json,
                        area.equipment_ids,
                    )
                    .await
                    {
                        Ok(data) => HttpResponse::Ok().json(data),

                        Err(error) => {
                            eprintln!("Ошибка обработки простоев: {}", error);

                            HttpResponse::InternalServerError().json(
                                serde_json::json!({
                                    "error": "Ошибка обработки данных"
                                }),
                            )
                        }
                    }
                }

                Err(error) => {
                    eprintln!("Ошибка парсинга JSON ТОИР: {}", error);

                    HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Некорректный ответ ТОИР"
                        }),
                    )
                }
            }
        }

        Err(error) => {
            eprintln!("Ошибка подключения к ТОИР: {}", error);

            HttpResponse::BadGateway().json(
                serde_json::json!({
                    "error": "ТОИР не отвечает"
                }),
            )
        }
    }
}*/

// ________________________________________________________________________________
#[post("/api/monitoring/{area}/downtimes/filter")]
async fn filter_monitoring_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<strdn::GetDowntimesRequest>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ
    // =========================================================
    if pool.local_mode {
        // println!("🔧 ЛОКАЛЬНЫЙ РЕЖИМ: Фильтрация простоев мониторинга для {}", area.name);
        
        match db::get_monitoring_downtimes_local(&pool, area).await {
            Ok(data) => {
                let filter = body.into_inner();
                
                let filtered_data = if let Some(start_date) = filter.Filter_StartDate {
                    let date_str = &start_date[0..8];
                    data.into_iter()
                        .filter(|item| {
                            if let Some(dt) = &item.StartDateTime {
                                dt.starts_with(date_str)
                            } else {
                                false
                            }
                        })
                        .collect()
                } else {
                    data
                };
                
                // println!("✅ Загружено {} записей для {}", filtered_data.len(), area.name);
                HttpResponse::Ok().json(filtered_data)
            }
            Err(error) => {
                eprintln!("❌ Ошибка загрузки локальных данных: {}", error);
                HttpResponse::InternalServerError().json(
                    serde_json::json!({
                        "error": "Ошибка загрузки данных"
                    })
                )
            }
        }
    } else {
        // РЕЖИМ ТОИР
        let mut filter = body.into_inner();

        filter.Filter_DepartmentID = Some(area.department_id.to_string());
        filter.WithEAMDowntimes = Some(1);
        filter.ShowDeleted = Some(0);
        filter.MaxRows = Some(50);

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
            .post(format!("{}/get_downtime_list", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&filter)
            .send()
            .await;

        match post_res {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("ТОИР вернул статус: {}", response.status());

                    return HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Ошибка внешнего сервиса"
                        }),
                    );
                }

                match response.json::<Value>().await {
                    Ok(json) => {
                        match db::get_downtimes_vec_jsonn_by_equipment(
                            &pool,
                            json,
                            area.equipment_ids,
                        )
                        .await
                        {
                            Ok(data) => HttpResponse::Ok().json(data),

                            Err(error) => {
                                eprintln!("Ошибка обработки простоев: {}", error);

                                HttpResponse::InternalServerError().json(
                                    serde_json::json!({
                                        "error": "Ошибка обработки данных"
                                    }),
                                )
                            }
                        }
                    }

                    Err(error) => {
                        eprintln!("Ошибка парсинга JSON ТОИР: {}", error);

                        HttpResponse::BadGateway().json(
                            serde_json::json!({
                                "error": "Некорректный ответ ТОИР"
                            }),
                        )
                    }
                }
            }

            Err(error) => {
                eprintln!("Ошибка подключения к ТОИР: {}", error);

                HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "ТОИР не отвечает"
                    }),
                )
            }
        }
    }
}

/*#[post("/api/monitoring/{area}/downtimes/filter")]
async fn filter_monitoring_downtimes(
    pool: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<strdn::GetDowntimesRequest>,
) -> impl Responder {
    let area_slug = path.into_inner();

    let Some(area) = db::find_monitoring_area(&area_slug) else {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "error": "Неизвестный участок"
            }),
        );
    };

    let mut filter = body.into_inner();

    filter.Filter_DepartmentID = Some(area.department_id.to_string());
    filter.WithEAMDowntimes = Some(1);
    filter.ShowDeleted = Some(0);
    filter.MaxRows = Some(50);

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
        .post(format!("{}/get_downtime_list", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
        .json(&filter)
        .send()
        .await;

    match post_res {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("ТОИР вернул статус: {}", response.status());

                return HttpResponse::BadGateway().json(
                    serde_json::json!({
                        "error": "Ошибка внешнего сервиса"
                    }),
                );
            }

            match response.json::<Value>().await {
                Ok(json) => {
                    match db::get_downtimes_vec_jsonn_by_equipment(
                        &pool,
                        json,
                        area.equipment_ids,
                    )
                    .await
                    {
                        Ok(data) => HttpResponse::Ok().json(data),

                        Err(error) => {
                            eprintln!("Ошибка обработки простоев: {}", error);

                            HttpResponse::InternalServerError().json(
                                serde_json::json!({
                                    "error": "Ошибка обработки данных"
                                }),
                            )
                        }
                    }
                }

                Err(error) => {
                    eprintln!("Ошибка парсинга JSON ТОИР: {}", error);

                    HttpResponse::BadGateway().json(
                        serde_json::json!({
                            "error": "Некорректный ответ ТОИР"
                        }),
                    )
                }
            }
        }

        Err(error) => {
            eprintln!("Ошибка подключения к ТОИР: {}", error);

            HttpResponse::BadGateway().json(
                serde_json::json!({
                    "error": "ТОИР не отвечает"
                }),
            )
        }
    }
}*/

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
    // ==== мониторинг ====
    cfg.service(render_monitoring_page);
    cfg.service(get_area_monitoring);
    cfg.service(get_monitoring_downtimes);
    cfg.service(filter_monitoring_downtimes);
}