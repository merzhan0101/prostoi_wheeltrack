use crate::data_base::db_downtimes as db;
use crate::structures::struct_downtimes as strdn;
use reqwest::Client;
use serde_json::Value;
use crate::structures::struct_pool_client::AppState;
use crate::areas::find_area;
use crate::infra::struct_template::{AllDowntimesTemplate, DowntimesTemplate, MonitoringTemplate};
use actix_web::{HttpResponse, Responder, get, post, web};
use askama::Template;
use uuid::Uuid;
use chrono::Local;
use std::fs;
use std::path::Path;

// ________________________________________________________________________________
async fn open_pdf() -> HttpResponse {
    // Путь к файлу относительно корня проекта
    let pdf_path = Path::new("static/instructions/instructionDowntimeZU.pdf");
    
    match fs::read(pdf_path) {
        Ok(content) => HttpResponse::Ok()
            .content_type("application/pdf")
            .body(content),
        Err(e) => {
            eprintln!("Ошибка чтения PDF: {}", e);
            HttpResponse::NotFound()
                .content_type("text/plain; charset=utf-8")
                .body("Инструкция не найдена")
        }
    }
}

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
// #[post("/api/update_downtime")]
// async fn update_downtime(
//     pool: web::Data<AppState>,
//     body: web::Json<serde_json::Value>,
// ) -> impl Responder {
//     let payload = body.into_inner();

//     let eamid = payload
//         .get("EAMID")
//         .and_then(|value| value.as_str())
//         .unwrap_or("");

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
    // ВАЛИДАЦИЯ: начало простоя не может быть позже конца
    // =========================================================
    let start_date = payload
        .get("StartDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let end_date = payload
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // EndDateTime может быть пустым (простой ещё идёт) — тогда не проверяем.
    // Формат "YYYYMMDDHHMMSS" одинаковой длины — строковое сравнение работает как временное.
    if !start_date.is_empty() && !end_date.is_empty() && start_date > end_date {
        return HttpResponse::BadRequest().json(
            serde_json::json!({
                "Posted": 0,
                "ErrorMessage": "Дата и время начала простоя не могут быть позже даты и времени окончания"
            })
        );
    }


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

        // =========================================================
        // ГОТОВИМ ОТДЕЛЬНЫЙ PAYLOAD ДЛЯ ТОИР
        // =========================================================
        let mut toir_payload = update_payload.clone();
        if let Some(object) = toir_payload.as_object_mut() {
            object.remove("Area");
            object.remove("DepartmentIDs");
        }

        // Пробуем явно передать флаги, которых нет в нашем payload,
        // но которые есть в структуре документа ТОИР (Data.Deleted, Data.Posted)
        toir_payload["Deleted"] = serde_json::json!(false);
        toir_payload["Posted"] = serde_json::json!(true);

        eprintln!(
            "📤 Отправка в ТОИР (edit, тест с Deleted/Posted): {}",
            serde_json::to_string_pretty(&toir_payload).unwrap_or_default()
        );

        // Подтягиваем актуальную EAMVersion документа у ТОИР —
        // без неё модуль ТОИР падает на приведении к Булево.
        if let Some(version) = get_version_from_toir(&client_http, &pool, eamid).await {
            toir_payload["EAMVersion"] = serde_json::json!(version);
        } else {
            eprintln!("⚠️ Не удалось получить EAMVersion для {} — пробуем без версии", eamid);
        }

        eprintln!(
            "📤 Отправка в ТОИР (edit): {}",
            serde_json::to_string_pretty(&toir_payload).unwrap_or_default()
        );

        let response = match client_http
            .post(format!("{}/update_downtime", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&toir_payload)
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
            Ok(text) => text,
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

                    // Обновляем локальную БД — здесь используем ПОЛНЫЙ payload
                    // (с Area/DepartmentIDs), локальной базе они не мешают
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

    /*if !eamid.is_empty() {
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
    }*/


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
            let error_msg = result_http
                .get("ErrorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("Неизвестная ошибка");

            eprintln!(
                "❌ ТОИР отклонил создание простоя. EquipmentID={}, Ответ ТОИР: {:#?}",
                equipment_id, result_http
            );

            return HttpResponse::Ok().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": format!("❌ ТОИР не принял простой: {}", error_msg)
                }),
            );
        }

        /*if posted != 1 {
            return HttpResponse::Ok().json(
                serde_json::json!({
                    "Posted": 0,
                    "ErrorMessage": "ТОИР не принял простой"
                }),
            );
        }*/

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

/*async fn get_version_from_toir(
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
}*/

async fn get_version_from_toir(
    client: &Client,
    pool: &AppState,
    eamid: &str,
) -> Option<String> {
    let request = serde_json::json!({
        "EAMID": eamid
    });

    let response = match client
        .post(format!("{}/get_downtime", pool.toir_url))
        .basic_auth(&pool.toir_login, Some(&pool.toir_password))
        .json(&request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Ошибка запроса к ТОИР (get_version): {}", error);
            return None;
        }
    };

    let status = response.status();

    // Читаем как ТЕКСТ, а не сразу как JSON — чтобы видеть реальный
    // ответ ТОИР, даже если это не валидный JSON (как с ошибкой 511)
    let response_text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            eprintln!("Ошибка чтения ответа ТОИР (get_version): {}", error);
            return None;
        }
    };

    eprintln!(
        "🔍 /get_downtime для EAMID={} — статус: {}, сырой ответ:\n{}",
        eamid, status, response_text
    );

    if !status.is_success() {
        return None;
    }

    match serde_json::from_str::<serde_json::Value>(&response_text) {
        Ok(json) => {
            // Печатаем ВСЕ ключи в Data, чтобы увидеть реальную структуру
            if let Some(data) = json.get("Data") {
                eprintln!(
                    "🔍 Полный объект Data от ТОИР:\n{}",
                    serde_json::to_string_pretty(data).unwrap_or_default()
                );
            }

            let version = json
                .get("Data")
                .and_then(|d| d.get("EAMVersion"))
                .and_then(|v| v.as_str())
                .or_else(|| json.get("EAMVersion").and_then(|v| v.as_str()));

            version.map(|s| s.to_string())
        }
        Err(error) => {
            eprintln!("Ошибка парсинга JSON (get_version): {}", error);
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
        // request.MaxRows = Some(50);
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

// ________________________________________________________________________________
// ПРОСТОИ ВСЕХ УЧАСТКОВ ===========================================================

/// Без даты — последние 50 простоев по всем участкам.
const ALL_DEFAULT_LIMIT: usize = 50;
/// С датой — все простои дня; лимит только как предохранитель.
const ALL_DAY_LIMIT: usize = 500;

const ALL_TOIR_PAGE_SIZE: i32 = 200;
/// Предохранитель: не больше стольких страниц за один запрос.
const ALL_TOIR_MAX_PAGES: usize = 10;
/// «Широкое» окно дат для показа последних записей независимо от дня.
const ALL_WIDE_START: &str = "20000101000000";
const ALL_WIDE_END: &str = "20991231235959";

/// Выгружает из ТОИР все простои подразделения (страницами), оставляет нужные
/// и сортирует по началу (новые сверху). Лимит применяет вызывающий код.
///
/// Порядок и лимит ТОИР нам не гарантированы, поэтому сортируем сами.
/// `day` (`YYYYMMDD`) — оставить только простои, начавшиеся в этот день
/// (ТОИР при фильтре по дате добавляет ещё и незакрытые простои других дней).
async fn all_fetch_toir_rows(
    pool: &AppState,
    base: &strdn::GetDowntimesRequest,
    department_id: &str,
    day: Option<&str>,
) -> Result<Vec<Value>, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Ошибка создания HTTP-клиента: {e}"))?;

    let mut rows: Vec<Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for _ in 0..ALL_TOIR_MAX_PAGES {
        let mut request = base.clone();
        request.Filter_DepartmentID = Some(department_id.to_string());
        request.WithEAMDowntimes = Some(1);
        request.ShowDeleted = Some(0);
        request.MaxRows = Some(ALL_TOIR_PAGE_SIZE);
        request.Offset = Some(seen.len() as i32);

        let response = client
            .post(format!("{}/get_downtime_list", pool.toir_url))
            .basic_auth(&pool.toir_login, Some(&pool.toir_password))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("ТОИР не отвечает: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("ТОИР вернул статус {}", response.status()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Некорректный ответ ТОИР: {e}"))?;

        let page = json
            .get("Data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if page.is_empty() {
            break;
        }

        // Если ТОИР игнорирует Offset и отдаёт то же самое — не зацикливаемся.
        let mut added = 0;
        for item in page {
            let id = item
                .get("EAMID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            if seen.insert(id) {
                rows.push(item);
                added += 1;
            }
        }
        if added == 0 {
            break;
        }
    }

    rows.retain(|item| {
        let same_department = item
            .get("DepartmentID")
            .and_then(|v| v.as_str())
            .is_some_and(|d| d.eq_ignore_ascii_case(department_id));

        let same_day = match day {
            Some(day) => item
                .get("StartDateTime")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.starts_with(day)),
            None => true,
        };

        same_department && same_day
    });

    // StartDateTime в формате YYYYMMDDHHMMSS — строки сравниваются как даты.
    rows.sort_by(|a, b| {
        let key = |v: &Value| {
            v.get("StartDateTime")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string()
        };
        key(b).cmp(&key(a))
    });

    Ok(rows)
}

/// Принадлежит ли строка ТОИР одному из перечисленных единиц оборудования.
fn all_row_has_equipment(item: &Value, equipment_ids: &[&str]) -> bool {
    let equipment = item
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    equipment_ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case(equipment))
}

#[derive(serde::Deserialize)]
struct AllDowntimesQuery {
    /// `YYYYMMDD` или `YYYY-MM-DD`
    date: Option<String>,
}

#[get("/all-downtimes")]
async fn render_all_downtimes_page() -> impl Responder {
    match AllDowntimesTemplate::new().render() {
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

/// Разбор параметра `date`: `Ok(None)` — даты нет, `Err` — некорректная дата.
fn parse_all_day(date: Option<&str>) -> Result<Option<String>, ()> {
    match date.map(|d| d.replace('-', "")) {
        Some(d) if d.is_empty() => Ok(None),
        Some(d) if d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit()) => Ok(Some(d)),
        Some(_) => Err(()),
        None => Ok(None),
    }
}

/// Простои всех участков: без даты — последние 50, с датой — все за день.
/// Ошибка — уже готовый HTTP-ответ.
async fn load_all_downtimes(
    pool: &AppState,
    day: Option<&str>,
) -> Result<Vec<strdn::AreaDowntime>, HttpResponse> {
    let limit = if day.is_some() { ALL_DAY_LIMIT } else { ALL_DEFAULT_LIMIT };

    if pool.local_mode {
        return db::get_all_areas_downtimes_local(pool, day, limit)
            .await
            .map_err(|error| {
                eprintln!("❌ Ошибка загрузки простоев всех участков: {}", error);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Ошибка загрузки данных"
                }))
            });
    }

    // Режим ТОИР: одна выгрузка на подразделение, затем деление по участкам.
    let (from, to) = match day {
        Some(d) => (format!("{d}000000"), format!("{d}235959")),
        None => (ALL_WIDE_START.to_string(), ALL_WIDE_END.to_string()),
    };

    let mut departments: Vec<&str> = db::MONITORING_AREAS
        .iter()
        .map(|area| area.department_id)
        .collect();
    departments.sort_unstable();
    departments.dedup();

    let mut result: Vec<strdn::AreaDowntime> = Vec::new();

    for department_id in departments {
        let request = strdn::GetDowntimesRequest {
            Filter_StartDate: Some(from.clone()),
            Filter_EndDate: Some(to.clone()),
            ..Default::default()
        };

        let rows = all_fetch_toir_rows(pool, &request, department_id, day)
            .await
            .map_err(|error| {
                eprintln!("Ошибка загрузки простоев из ТОИР: {}", error);
                HttpResponse::BadGateway().json(serde_json::json!({
                    "error": "ТОИР не отвечает или вернул некорректный ответ"
                }))
            })?;

        for area in db::MONITORING_AREAS
            .iter()
            .filter(|area| area.department_id.eq_ignore_ascii_case(department_id))
        {
            // rows уже отсортированы: новые сверху.
            let area_rows: Vec<Value> = rows
                .iter()
                .filter(|item| all_row_has_equipment(item, area.equipment_ids))
                .take(limit)
                .cloned()
                .collect();

            let json = serde_json::json!({ "Data": area_rows });

            let items = db::get_downtimes_vec_jsonn_by_equipment(pool, json, area.equipment_ids)
                .await
                .map_err(|error| {
                    eprintln!("Ошибка обработки простоев: {}", error);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Ошибка обработки данных"
                    }))
                })?;

            result.extend(items.into_iter().map(|downtime| strdn::AreaDowntime {
                AreaName: area.name.to_string(),
                Downtime: downtime,
            }));
        }
    }

    result.sort_by(|a, b| b.Downtime.StartDateTime.cmp(&a.Downtime.StartDateTime));
    result.truncate(limit);

    Ok(result)
}

#[get("/api/all-downtimes")]
async fn get_all_downtimes(
    pool: web::Data<AppState>,
    query: web::Query<AllDowntimesQuery>,
) -> impl Responder {
    let Ok(day) = parse_all_day(query.date.as_deref()) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Некорректная дата"
        }));
    };

    match load_all_downtimes(&pool, day.as_deref()).await {
        Ok(data) => HttpResponse::Ok().json(data),
        Err(response) => response,
    }
}

// ЭКСПОРТ В EXCEL ===================================================================

/// `YYYYMMDDHHMMSS` -> дата-время для Excel.
fn parse_toir_datetime(value: Option<&str>) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(value?, "%Y%m%d%H%M%S").ok()
}

fn build_all_downtimes_xlsx(
    items: &[strdn::AreaDowntime],
) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    use chrono::{Datelike, Timelike};
    use rust_xlsxwriter::{Color, ExcelDateTime, Format, FormatAlign, FormatBorder, Workbook};

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Простои")?;

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE2E8F0))
        .set_border_bottom(FormatBorder::Thin)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_text_wrap();
    let text_format = Format::new()
        .set_text_wrap()
        .set_align(FormatAlign::Top);
    let datetime_format = Format::new()
        .set_num_format("dd.mm.yyyy hh:mm")
        .set_align(FormatAlign::Left)
        .set_align(FormatAlign::Top);

    let headers = [
        "Участок",
        "Документ",
        "Оборудование",
        "Начало простоя",
        "Конец простоя",
        "Длительность",
        "Тип простоя",
        "Автор",
        "Комментарий",
    ];
    let widths = [26.0, 16.0, 38.0, 18.0, 18.0, 14.0, 26.0, 22.0, 50.0];

    for (col, (title, width)) in headers.iter().zip(widths).enumerate() {
        sheet.write_string_with_format(0, col as u16, *title, &header_format)?;
        sheet.set_column_width(col as u16, width)?;
    }

    for (index, item) in items.iter().enumerate() {
        let row = (index + 1) as u32;
        let d = &item.Downtime;

        sheet.write_string_with_format(row, 0, &item.AreaName, &text_format)?;
        sheet.write_string_with_format(row, 1, d.EAMDocName.as_deref().unwrap_or(""), &text_format)?;
        sheet.write_string_with_format(row, 2, d.EquipmentName.as_deref().unwrap_or(""), &text_format)?;

        let start = parse_toir_datetime(d.StartDateTime.as_deref());
        let end = parse_toir_datetime(d.EndDateTime.as_deref());

        for (col, value) in [(3u16, start), (4u16, end)] {
            if let Some(value) = value {
                let excel = ExcelDateTime::from_ymd(
                    value.year() as u16,
                    value.month() as u8,
                    value.day() as u8,
                )?
                .and_hms(value.hour() as u16, value.minute() as u8, value.second() as f64)?;

                sheet.write_datetime_with_format(row, col, &excel, &datetime_format)?;
            }
        }

        if let (Some(start), Some(end)) = (start, end) {
            let minutes = (end - start).num_minutes();
            if minutes >= 0 {
                sheet.write_string_with_format(
                    row,
                    5,
                    format!("{} ч {} мин", minutes / 60, minutes % 60),
                    &text_format,
                )?;
            }
        } else if start.is_some() {
            sheet.write_string_with_format(row, 5, "не завершён", &text_format)?;
        }

        sheet.write_string_with_format(row, 6, d.DowntimeTypeName.as_deref().unwrap_or(""), &text_format)?;
        sheet.write_string_with_format(row, 7, d.Author.as_deref().unwrap_or(""), &text_format)?;
        sheet.write_string_with_format(row, 8, d.Comment.as_deref().unwrap_or(""), &text_format)?;
    }

    sheet.set_row_height(0, 24)?;
    sheet.set_freeze_panes(1, 0)?;
    sheet.autofilter(0, 0, items.len() as u32, (headers.len() - 1) as u16)?;

    workbook.save_to_buffer()
}

#[get("/api/all-downtimes/export")]
async fn export_all_downtimes(
    pool: web::Data<AppState>,
    query: web::Query<AllDowntimesQuery>,
) -> impl Responder {
    let Ok(day) = parse_all_day(query.date.as_deref()) else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Некорректная дата"
        }));
    };

    let items = match load_all_downtimes(&pool, day.as_deref()).await {
        Ok(items) => items,
        Err(response) => return response,
    };

    let bytes = match build_all_downtimes_xlsx(&items) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Ошибка формирования Excel: {}", error);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Ошибка формирования файла Excel"
            }));
        }
    };

    let filename = match &day {
        Some(d) => format!("downtimes_{}-{}-{}.xlsx", &d[0..4], &d[4..6], &d[6..8]),
        None => "downtimes_last_50.xlsx".to_string(),
    };

    HttpResponse::Ok()
        .content_type("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
        .body(bytes)
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
    // ==== мониторинг ====
    cfg.service(render_monitoring_page);
    cfg.service(get_area_monitoring);
    cfg.service(get_monitoring_downtimes);
    cfg.service(filter_monitoring_downtimes);
    // ==== простои всех участков ====
    cfg.service(render_all_downtimes_page);
    cfg.service(get_all_downtimes);
    cfg.service(export_all_downtimes);
    // ==== инструкция ====
    cfg.route("/instruction_downtime", web::get().to(open_pdf));
}