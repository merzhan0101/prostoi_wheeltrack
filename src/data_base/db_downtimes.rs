use std::println;

use crate::data_base::id_search as sh;
use crate::structures::struct_downtimes as dsr;
use anyhow::Result;
use chrono::NaiveDateTime;
use futures_util::TryStreamExt;
use serde_json::Value;
use tiberius::{Query, QueryItem};
use uuid::Uuid;

use crate::structures::struct_pool_client::AppState;

// ________________________________________________________________________________
pub async fn get_equipment(
    pool: &AppState,
    equipment_name: Option<String>,
    department_id: &str,
) -> Result<Vec<dsr::SearchNameDepartResult>> {
    let mut client = pool.pool_raport.get().await?;

    let search_text = format!(
        "%{}%",
        equipment_name.unwrap_or_default()
    );

    let department_bin = Uuid::parse_str(department_id)?
        .into_bytes()
        .to_vec();

    let mut query = Query::new(
        "
        SELECT
            e.Id,
            e.Name
        FROM DTEquipments e
        WHERE e.Name LIKE @P1
        AND e.DepartmentID = @P2
        AND e.Marked = 0
        AND NOT EXISTS (
            SELECT 1
            FROM Downtimes d
            WHERE d.EquipmentID = e.Id
                AND d.EndDateTime IS NULL
        )
        ORDER BY e.Name
        ",
    );

    // let mut query = Query::new(
    //     "
    //     SELECT TOP 10
    //         Id,
    //         Name
    //     FROM DTEquipments
    //     WHERE Name LIKE @P1
    //       AND DepartmentID = @P2
    //       AND Marked = 0
    //     ORDER BY Name
    //     ",
    // );

    query.bind(search_text);
    query.bind(department_bin);

    let mut result = Vec::new();
    let mut stream = query.query(&mut client).await?;

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            result.push(dsr::SearchNameDepartResult {
                id: row
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| {
                        Uuid::from_slice(bytes)
                            .ok()
                            .map(|uuid| uuid.to_string())
                    }),

                name: row
                    .get::<&str, _>("Name")
                    .map(String::from),
            });
        }
    }

    Ok(result)
}

// ________________________________________________________________________________
pub async fn get_name_worker(
    pool: &AppState,
    worker_name: Option<String>,
    place_id: i32,
) -> Result<Vec<dsr::SearchNameWorkerResult>> {
    let mut client = pool.pool_wt.get().await?;

    let like_query = format!(
        "%{}%",
        worker_name.unwrap_or_default()
    );

    let mut query = Query::new(
        "
        SELECT
            Id,
            FIO
        FROM Worker
        WHERE FIO LIKE @P1
          AND Place_id = @P2
          AND (Post_id = 1 or Post_id = 3)
        ORDER BY FIO
        ",
    );

    query.bind(like_query);
    query.bind(place_id);

    let mut result = Vec::new();
    let mut stream = query.query(&mut client).await?;

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            result.push(
                dsr::SearchNameWorkerResult {
                    id: row.get("Id"),

                    fio: row
                        .get::<&str, _>("FIO")
                        .map(String::from),
                },
            );
        }
    }

    Ok(result)
}

// ________________________________________________________________________________
pub async fn get_types(pool: &AppState) -> Result<Vec<dsr::DataTypes>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = dsr::DataTypes::default();

    let mut data_result: Vec<dsr::DataTypes> = Vec::new();

    // let select = Query::new("SELECT Id, Name FROM DowntimesTypes");
    let select = Query::new(
        "
        SELECT
            t.Id,
            t.Name,
            c.IsEmergency
        FROM DowntimesTypes t
        LEFT JOIN DowntimesTypesClassification c ON c.DowntimeTypeID = t.Id
        ",
    );

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.id = r
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| Uuid::from_slice(bytes).ok().map(|u| u.to_string()));
                data.name = r.get("Name").map(|s: &str| s.to_string());
                data.is_emergency = r.get::<bool, _>("IsEmergency"); //add
                data_result.push(data.clone());
            }
        }
    }

    Ok(data_result)
}

// ________________________________________________________________________________
pub async fn has_active_downtime(
    pool: &AppState,
    equipment_id: &str,
) -> Result<bool> {
    let mut client = pool.pool_raport.get().await?;

    let equipment_id_bin = Uuid::parse_str(equipment_id)?
        .into_bytes()
        .to_vec();

    let mut query = Query::new(
        "
        SELECT TOP 1 1
        FROM Downtimes
        WHERE EquipmentID = @P1
          AND EndDateTime IS NULL
        ",
    );

    query.bind(equipment_id_bin);

    let mut stream = query.query(&mut client).await?;

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(_) = item {
            return Ok(true);
        }
    }

    Ok(false)
}

// ________________________________________________________________________________
pub async fn update_insert(pool: &AppState, body: Value, result_http: Value) -> Result<()> {
    let mut client = pool.pool_raport.get().await?;

    // =========================================================
    // ЛОКАЛЬНЫЙ РЕЖИМ - отключаем FOREIGN KEY
    // =========================================================
    let is_local_mode = body.get("_local_mode").and_then(|v| v.as_bool()).unwrap_or(false);
    
    if is_local_mode {
        let disable_query = "ALTER TABLE Downtimes NOCHECK CONSTRAINT FK_Downtimes_Equip";
        if let Err(e) = client.execute(disable_query, &[]).await {
            eprintln!("⚠️ Не удалось отключить FOREIGN KEY: {}", e);
        }
    }

    // =========================================================
    // ПОДГОТОВКА ДАННЫХ
    // =========================================================
    let eam_version = result_http
        .get("EAMVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bin_id = result_http
        .get("EAMID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

    let eam_doc = result_http
        .get("EAMDocName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Парсим даты
    let start_date_str = body
        .get("StartDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let dt = if !start_date_str.is_empty() {
        match NaiveDateTime::parse_from_str(start_date_str, "%Y%m%d%H%M%S") {
            Ok(dt) => dt,
            Err(e) => {
                eprintln!("❌ Ошибка парсинга StartDateTime '{}': {}", start_date_str, e);
                chrono::Local::now().naive_local()
            }
        }
    } else {
        chrono::Local::now().naive_local()
    };

    let dt1 = body
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S").ok());

    // EquipmentID
    let equipment_id = body
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let equipment_id_bin = if !equipment_id.is_empty() {
        match Uuid::parse_str(equipment_id) {
            Ok(uuid) => uuid.into_bytes().to_vec(),
            Err(e) => {
                eprintln!("❌ Ошибка парсинга EquipmentID '{}': {}", equipment_id, e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // DepartmentID
    let department_id = body
        .get("DepartmentID")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let department_id_bin: Option<Vec<u8>> = if !department_id.is_empty() {
        match Uuid::parse_str(department_id) {
            Ok(uuid) => Some(uuid.into_bytes().to_vec()),
            Err(_) => None
        }
    } else {
        None
    };

    let downtime_id_bin = body
        .get("DowntimeTypeID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

    let is_empty = body
        .get("EAMID")
        .and_then(|v| v.as_str())
        .map_or(true, |s| s.is_empty());

    let author = body
        .get("Author")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let comment = body
        .get("Comment")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // =========================================================
    // ВЫПОЛНЯЕМ ЗАПРОС (INSERT или UPDATE)
    // =========================================================
    let result = if is_empty {
        // ВСТАВКА
        let insert = "
            INSERT INTO Downtimes 
            (Id, EquipmentID, DepartmentID, DowntimeTypeID, StartDateTime, EndDateTime, Author, Comment, EAMDocName, EAMVersion)
            VALUES(@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10)
        ";
        
        client
            .execute(
                insert,
                &[
                    &bin_id,
                    &equipment_id_bin,
                    &department_id_bin,
                    &downtime_id_bin,
                    &dt,
                    &dt1,
                    &author,
                    &comment,
                    &eam_doc,
                    &eam_version,
                ],
            )
            .await
    } else {
        // ОБНОВЛЕНИЕ
        let update = "
            UPDATE Downtimes
            SET
                EquipmentID = @P1,
                DepartmentID = COALESCE(@P2, DepartmentID),
                DowntimeTypeID = @P3,
                StartDateTime = @P4,
                EndDateTime = @P5,
                Author = @P6,
                Comment = @P7
            WHERE Id = @P8
        ";

        client
            .execute(
                update,
                &[
                    &equipment_id_bin,
                    &department_id_bin,
                    &downtime_id_bin,
                    &dt,
                    &dt1,
                    &author,
                    &comment,
                    &bin_id,
                ],
            )
            .await
    };

    // =========================================================
    // ВКЛЮЧАЕМ FOREIGN KEY (только в локальном режиме)
    // =========================================================
    if is_local_mode {
        let enable_query = "ALTER TABLE Downtimes CHECK CONSTRAINT FK_Downtimes_Equip";
        if let Err(e) = client.execute(enable_query, &[]).await {
            eprintln!("⚠️ Не удалось включить FOREIGN KEY: {}", e);
        }
    }

    // =========================================================
    // ВОЗВРАЩАЕМ РЕЗУЛЬТАТ
    // =========================================================
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("❌ Ошибка вставки/обновления в БД: {}", e);
            Err(anyhow::anyhow!("Ошибка БД: {}", e))
        }
    }
}

// ____________________________________________________________________________________
pub async fn get_downtimes_vec_jsonn(
    pool: &AppState,
    json: Value,
    department_id: &str,
) -> Result<Vec<dsr::DowntimesRefactor>> {
    let mut client = pool.pool_raport.get().await?;
    
    // let state = pool.pool_raport.state();
    // println!(
    //     "СТАТУС ПУЛА в функции: Всего соединений: {}, Открыто прямо сейчас: {}",
    //     state.connections,      // сколько физически открыто в БД
    //     state.idle_connections, // сколько из них СВОБОДНО прямо сейчас   // сколько потоков СЕЙЧАС ждут в очереди (если > 0, мы в дедлоке)
    // );

    let data_array = json
        .get("Data")
        .and_then(|v| v.as_array())
        .ok_or("Поле 'Data' отсутствует или не является массивом")
        .unwrap();

    let mut data = dsr::DowntimesRefactor::default();

    let mut data_send: Vec<dsr::DowntimesRefactor> = Vec::new();

    for item in data_array {
        let item_department_id = item
            .get("DepartmentID")
            .and_then(|value| value.as_str())
            .unwrap_or("");

        // Пропускаем простои других участков.
        if !item_department_id.eq_ignore_ascii_case(
            department_id
        ) {
            continue;
        }

        data.EAMID = item
            .get("EAMID")
            .and_then(|value| value.as_str())
            .map(String::from);

        data.EAMDocName = item
            .get("EAMDocName")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.EAMDowntime = item
            .get("EAMDowntime")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        data.EquipmentName = sh::equip(
            &mut client,
            item.get("EquipmentID")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap(),
        )
        .await
        .unwrap();
        data.DepartmentName = sh::depart(
            &mut client,
            item.get("DepartmentID")
                .and_then(|v| v.as_str())
                // .map(String::from)
                .unwrap(),
        )
        .await
        .unwrap();

        data.StartDateTime = item
            .get("StartDateTime")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.EndDateTime = item
            .get("EndDateTime")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Duration = item
            .get("Duration")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.DowntimeTypeName = sh::dtypes(
            &mut client,
            item.get("DowntimeTypeID").and_then(|v| v.as_str()).unwrap(),
        )
        .await
        .unwrap_or(None);

        data.Author = item
            .get("Author")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Comment = item
            .get("Comment")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Deleted = item
            .get("Deleted")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        data_send.push(data.clone());
    }

    Ok(data_send)
}

// ________________________________________________________________________________
pub async fn get_downtimes_vec_jsonn_by_equipment(
    pool: &AppState,
    json: Value,
    equipment_ids: &[&str],
) -> Result<Vec<dsr::DowntimesRefactor>> {
    let mut client = pool.pool_raport.get().await?;

    let data_array = json
        .get("Data")
        .and_then(|v| v.as_array())
        .ok_or("Поле 'Data' отсутствует или не является массивом")
        .unwrap();

    let mut data = dsr::DowntimesRefactor::default();

    let mut data_send: Vec<dsr::DowntimesRefactor> = Vec::new();

    for item in data_array {
        let item_equipment_id = item
            .get("EquipmentID")
            .and_then(|value| value.as_str())
            .unwrap_or("");

        // Пропускаем простои оборудования, не относящегося к этому
        // логическому участку мониторинга.
        if !equipment_ids
            .iter()
            .any(|id| id.eq_ignore_ascii_case(item_equipment_id))
        {
            continue;
        }

        data.EAMID = item
            .get("EAMID")
            .and_then(|value| value.as_str())
            .map(String::from);

        data.EAMDocName = item
            .get("EAMDocName")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.EAMDowntime = item
            .get("EAMDowntime")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        data.EquipmentName = sh::equip(
            &mut client,
            item.get("EquipmentID")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap(),
        )
        .await
        .unwrap();
        data.DepartmentName = sh::depart(
            &mut client,
            item.get("DepartmentID")
                .and_then(|v| v.as_str())
                .unwrap(),
        )
        .await
        .unwrap();

        data.StartDateTime = item
            .get("StartDateTime")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.EndDateTime = item
            .get("EndDateTime")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Duration = item
            .get("Duration")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.DowntimeTypeName = sh::dtypes(
            &mut client,
            item.get("DowntimeTypeID").and_then(|v| v.as_str()).unwrap(),
        )
        .await
        .unwrap_or(None);

        data.Author = item
            .get("Author")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Comment = item
            .get("Comment")
            .and_then(|v| v.as_str())
            .map(String::from);

        data.Deleted = item
            .get("Deleted")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        data_send.push(data.clone());
    }

    Ok(data_send)
}

// ________________________________________________________________________________
pub async fn get_data_downtime(
    pool: &AppState,
    json: Value,
    eamid: &str,
) -> Result<dsr::GetDowntime> {
    let mut client = pool.pool_raport.get().await?;

    let data = json.get("Data").unwrap();
    let mut data_toir = dsr::GetDowntime::default();

    data_toir.EAMID = Some(eamid.to_string());

    data_toir.EAMDowntime = data
        .get("EAMDowntime")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    data_toir.EquipmentID = data
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.EquipmentName = sh::equip(
        &mut client,
        data.get("EquipmentID")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap(),
    )
    .await
    .unwrap();

    data_toir.DepartmentID = data
        .get("DepartmentID")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.StartDateTime = data
        .get("StartDateTime")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.EndDateTime = data
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.Duration = data
        .get("Duration")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    data_toir.DowntimeTypeID = data
        .get("DowntimeTypeID")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.Author = data
        .get("Author")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.Comment = data
        .get("Comment")
        .and_then(|v| v.as_str())
        .map(String::from);

    data_toir.Posted = data
        .get("Posted")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    Ok(data_toir)
}

// ________________________________________________________________________________
pub async fn data_update(pool: &AppState, body: Value) -> Result<dsr::UpdateData> {
    let mut client = pool.pool_raport.get().await?;
    let mut data = dsr::UpdateData::default();

    data.EAMID = body.get("EAMID").and_then(|v| v.as_str()).map(String::from);

    // Получаем EAMVersion из БД
    if let Some(eamid) = &data.EAMID {
        let bin_id = Uuid::parse_str(eamid)
            .map(|u| u.into_bytes().to_vec())
            .unwrap_or_default();

        let mut select_version = Query::new("SELECT TOP 1 EAMVersion FROM Downtimes WHERE Id=@P1");
        select_version.bind(bin_id);

        let mut stream = select_version.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.EAMVersion = r.get(0).map(|s: &str| s.to_string());
                break;
            }
        }
    }

    // Заполняем остальные поля
    data.EquipmentID = body.get("EquipmentID").and_then(|v| v.as_str()).map(String::from);
    data.DowntimeTypeID = body.get("DowntimeTypeID").and_then(|v| v.as_str()).map(String::from);
    data.StartDateTime = body.get("StartDateTime").and_then(|v| v.as_str()).map(String::from);
    data.EndDateTime = body.get("EndDateTime").and_then(|v| v.as_str()).map(String::from);
    data.Author = body.get("Author").and_then(|v| v.as_str()).map(String::from);
    data.Comment = body.get("Comment").and_then(|v| v.as_str()).map(String::from);
    
    // Добавляем DepartmentID если есть
    data.DepartmentID = body.get("DepartmentID").and_then(|v| v.as_str()).map(String::from);
    
    // Добавляем Posted как false (или true, зависит от ТОИР)
    // data.Posted = Some("false".to_string());

    Ok(data)
}

// ________________________________________________________________________________
pub async fn get_name_department(
    pool: &AppState,
    name_depart: Option<String>,
) -> Result<Vec<dsr::DepartmentName>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = dsr::DepartmentName::default();

    let like_query = format!("%{}%", name_depart.unwrap_or("".to_string()));

    let mut data_result: Vec<dsr::DepartmentName> = Vec::new();

    // let mut select = Query::new("SELECT Id, Name FROM DTDepartments WHERE Marked = 0 and Name in ('Участок по ремонту оборудования проката', 'Участок по ремонту гидравлического оборудования', 'Отдел АСУП колесного производства')");
    let mut select = Query::new(
        "  SELECT Id, Name
                FROM DTDepartments
                WHERE Id IN (
                    0x40FE0563C04A11F0A48D00155D011305,
                    0xC3DF935DC16311ED816300155D16FE68,
                    0x2CB0C7E9604511EE80CB00155D07EA17
                );" //убрал первый Участок по ремонту оборудования проката - 0xF26321C79D6C11ED816300155D16FE68 (20.08.2026)
    );
    // select.bind(like_query);

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.id = r
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| Uuid::from_slice(bytes).ok().map(|u| u.to_string()));
                data.department = r.get("Name").map(|s: &str| s.to_string());
                data_result.push(data.clone());
            }
        }
    }

    Ok(data_result)
}


// ЛОКАЛЬНЫЙ РЕЖИМ =======================================================================================================

pub async fn get_downtime_local(
    pool: &AppState,
    eamid: &str,
) -> Result<dsr::GetDowntime> {
    let mut client = pool.pool_raport.get().await?;

    let bin_id = Uuid::parse_str(eamid)?
        .into_bytes()
        .to_vec();

    let query = "
        SELECT 
            d.Id as EAMID,
            d.EAMDocName,
            d.EAMVersion,
            d.EquipmentID,
            e.Name as EquipmentName,
            d.DepartmentID,
            dep.Name as DepartmentName,
            d.StartDateTime,
            d.EndDateTime,
            dt.Name as DowntimeTypeName,
            d.DowntimeTypeID,
            d.Author,
            d.Comment
        FROM Downtimes d
        LEFT JOIN DTEquipments e ON d.EquipmentID = e.Id
        LEFT JOIN DTDepartments dep ON d.DepartmentID = dep.Id
        LEFT JOIN DowntimesTypes dt ON d.DowntimeTypeID = dt.Id
        WHERE d.Id = @P1
    ";

    let mut stream = client.query(query, &[&bin_id]).await?;
    let mut data_toir = dsr::GetDowntime::default();

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            // EAMID
            if let Some(bytes) = row.get::<&[u8], _>("EAMID") {
                if let Ok(uuid) = Uuid::from_slice(bytes) {
                    data_toir.EAMID = Some(uuid.to_string());
                }
            }
            
            data_toir.EAMDocName = row.get("EAMDocName").map(|s: &str| s.to_string());
            data_toir.EAMVersion = row.get("EAMVersion").map(|s: &str| s.to_string());
            
            // EquipmentID
            if let Some(bytes) = row.get::<&[u8], _>("EquipmentID") {
                if let Ok(uuid) = Uuid::from_slice(bytes) {
                    data_toir.EquipmentID = Some(uuid.to_string());
                }
            }
            
            data_toir.EquipmentName = row.get("EquipmentName").map(|s: &str| s.to_string());
            
            // DepartmentID
            if let Some(bytes) = row.get::<&[u8], _>("DepartmentID") {
                if let Ok(uuid) = Uuid::from_slice(bytes) {
                    data_toir.DepartmentID = Some(uuid.to_string());
                }
            }
            
            // Форматируем даты
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("StartDateTime") {
                data_toir.StartDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("EndDateTime") {
                data_toir.EndDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            // Рассчитываем длительность
            if let (Some(start), Some(end)) = (
                row.get::<chrono::NaiveDateTime, _>("StartDateTime"),
                row.get::<chrono::NaiveDateTime, _>("EndDateTime")
            ) {
                let duration = end - start;
                data_toir.Duration = Some(duration.num_minutes() as i32);
            }
            
            data_toir.DowntimeTypeID = row.get("DowntimeTypeID")
                .and_then(|bytes: &[u8]| Uuid::from_slice(bytes).ok())
                .map(|uuid| uuid.to_string());
            
            data_toir.DowntimeTypeName = row.get("DowntimeTypeName").map(|s: &str| s.to_string());
            data_toir.Author = row.get("Author").map(|s: &str| s.to_string());
            data_toir.Comment = row.get("Comment").map(|s: &str| s.to_string());
            data_toir.Posted = Some(0); // В локальном режиме всегда 0
        }
    }

    Ok(data_toir)
}

pub async fn get_downtimes_local(
    pool: &AppState,
    department_id: &str,
) -> Result<Vec<dsr::DowntimesRefactor>> {
    let mut client = pool.pool_raport.get().await?;

    let department_bin = Uuid::parse_str(department_id)?
        .into_bytes()
        .to_vec();

    let query = "
        SELECT 
            d.Id as EAMID,
            d.EAMDocName,
            e.Name as EquipmentName,
            dep.Name as DepartmentName,
            d.StartDateTime,
            d.EndDateTime,
            dt.Name as DowntimeTypeName,
            d.Author,
            d.Comment
        FROM Downtimes d
        LEFT JOIN DTEquipments e ON d.EquipmentID = e.Id
        LEFT JOIN DTDepartments dep ON d.DepartmentID = dep.Id
        LEFT JOIN DowntimesTypes dt ON d.DowntimeTypeID = dt.Id
        WHERE d.DepartmentID = @P1
        ORDER BY d.StartDateTime DESC
        OFFSET 0 ROWS
        FETCH NEXT 50 ROWS ONLY
    ";

    let mut result = Vec::new();
    let mut stream = client.query(query, &[&department_bin]).await?;

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            let mut data = dsr::DowntimesRefactor::default();
            
            // EAMID
            if let Some(bytes) = row.get::<&[u8], _>("EAMID") {
                if let Ok(uuid) = Uuid::from_slice(bytes) {
                    data.EAMID = Some(uuid.to_string());
                }
            }
            
            data.EAMDocName = row.get("EAMDocName").map(|s: &str| s.to_string());
            data.EquipmentName = row.get("EquipmentName").map(|s: &str| s.to_string());
            data.DepartmentName = row.get("DepartmentName").map(|s: &str| s.to_string());
            
            // Даты
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("StartDateTime") {
                data.StartDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("EndDateTime") {
                data.EndDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            // Длительность
            if let (Some(start), Some(end)) = (
                row.get::<chrono::NaiveDateTime, _>("StartDateTime"),
                row.get::<chrono::NaiveDateTime, _>("EndDateTime")
            ) {
                let duration = end - start;
                let hours = duration.num_hours();
                let minutes = duration.num_minutes() % 60;
                data.Duration = Some(format!("{}ч {}м", hours, minutes));
            }
            
            data.DowntimeTypeName = row.get("DowntimeTypeName").map(|s: &str| s.to_string());
            data.Author = row.get("Author").map(|s: &str| s.to_string());
            data.Comment = row.get("Comment").map(|s: &str| s.to_string());
            data.Deleted = Some(0);
            data.EAMDowntime = None;

            result.push(data);
        }
    }

    Ok(result)
}


// МОНИТОРИНГ =======================================================================================================

// Список оборудования (и его порядок отображения) для страницы мониторинга
pub struct MonitoringArea {
    pub slug: &'static str,
    pub name: &'static str,
    pub department_id: &'static str,
    pub equipment_ids: &'static [&'static str],
}

pub const MONITORING_AREAS: &[MonitoringArea] = &[
    MonitoringArea {
        slug: "workpiece",
        name: "Заготовительный участок",
        department_id: "92b138d1-c131-11ed-8163-00155d16fe68",
        equipment_ids: &[
            "00ba56a9-be27-11ed-8163-00155d16fe68", // Стенд осмотра заготовок (штанг)
            "2189f58a-be27-11ed-8163-00155d16fe68", // Тележка г/п 20 т №1
            "3f883a54-c147-11ed-8163-00155d16fe68", // Тележка г/п 20 т №2
            "02e9d373-be24-11ed-8163-00155d16fe68", // Загрузочный стол (цепной конвейер)
            "7ffcb411-bc9a-11ed-8163-00155d16fe68", // Пильная установка LINSINGER №1
            "29cad886-c3bd-11ed-8163-00155d16fe68", // Пильная установка LINSINGER №2
            "9b7d1cf6-c222-11ed-8163-00155d16fe68", // Пильная установка LINSINGER №3
            "f176a2a7-be23-11ed-8163-00155d16fe68", // Подвижный рольганг
            "acff607c-be26-11ed-8163-00155d16fe68", // Портальный манипулятор Glama А1
        ],
    },
    MonitoringArea {
        slug: "ring_oven",
        name: "Кольцевая печь",
        department_id: "92b138cb-c131-11ed-8163-00155d16fe68",
        equipment_ids: &[
            "c0489326-cd25-11ed-8163-00155d16fe68", // Кольцевая печь
            "5930bdc5-cd1e-11ed-8163-00155d16fe68", // Транспортная система подачи и выдачи заготовок Krei-ma
            "4980dfc0-cd2b-11ed-8163-00155d16fe68", // ДОБАВИЛ -> Манипулятор загрузки кольцевой печи Glama B1
            "9bb0b344-cd3d-11ed-8163-00155d16fe68", // Манипулятор выгрузки кольцевой печи Glama B2
        ],
    },
    MonitoringArea {
        slug: "thermal_section",
        name: "Термоучасток",
        department_id: "92b138cb-c131-11ed-8163-00155d16fe68",
        equipment_ids: &[

            "bce3b454-ce17-11ed-8163-00155d16fe68", // Портальный манипулятор GLAMA D1
            "9eac762f-cd44-11ed-8163-00155d16fe68", // Транспортная система линии термообработки Krei-ma
            "abdc0a0d-ce1b-11ed-8163-00155d16fe68", // Портальный манипулятор GLAMA D2
            "bb85d7f9-cea3-11ed-8163-00155d16fe68", // Трансбордеры с системой гидравлики Vollert
            "9a3d5f30-ceb4-11ed-8163-00155d16fe68", // Печь закалочная
            "71fb5007-de76-11ed-8164-00155d16fe68", // Дымоудаление закалочной и отпускной печей
            "1f3d47e7-cee7-11ed-8163-00155d16fe68", // Манипулятор D3: Портал 3
            "511e46a0-cebf-11ed-8163-00155d16fe68", // Закалочная машина MSA №1
            "51b9fc6b-cecc-11ed-8163-00155d16fe68", // Закалочная машина MSA №2
            "800e3f37-cecc-11ed-8163-00155d16fe68", // Закалочная машина MSA №3
            "9176a33c-cecc-11ed-8163-00155d16fe68", // Закалочная машина MSA №4
            "32a14478-cebc-11ed-8163-00155d16fe68", // Печь отпускная
        ],
    },
    MonitoringArea {
        slug: "rolling",
        name: "Прессо-прокатный участок",
        department_id: "92b138d4-c131-11ed-8163-00155d16fe68",
        equipment_ids: &[
            "9dcbed00-c14a-11ed-8163-00155d16fe68", // Установка для удаления окалины SGGT
            "13d77b55-c172-11ed-8163-00155d16fe68", // Пресс МН-10000 Schuler
            "b1a155c0-c496-11ed-8163-00155d16fe68", // Система охлаждения и смазки Bemers для пресса МН-10000
            "8c5713a6-cc5f-11ed-8163-00155d16fe68", // Манипулятор Glama С1
            "dd75b23b-c177-11ed-8163-00155d16fe68", // Колесопрокатный стан Schuler
            "c3bef18e-c161-11ed-8163-00155d16fe68", // Манипулятор Glama С2
            "76bcf365-c173-11ed-8163-00155d16fe68", // Пресс МН-5000 Schuler
            "c92252da-cc7d-11ed-8163-00155d16fe68", // Шарнирный ленточный транспортер Ruez
            "79b46068-c49c-11ed-8163-00155d16fe68", // Система охлаждения и смазки Bemers для пресса МН-5000
            "722c5111-c174-11ed-8163-00155d16fe68", // Манипулятор Glama С3
            "3110dfaa-c4ad-11ed-8163-00155d16fe68", // Пресс маркировочный Stommel+Voos
            "3b831837-cc63-11ed-8163-00155d16fe68", // Устройство лазерное измерительное Mermec
            // Манипулятор Glama С0 из макета отсутствует в DTEquipments — не включён.
        ],
    },
];

pub fn find_monitoring_area(slug: &str) -> Option<&'static MonitoringArea> {
    MONITORING_AREAS.iter().find(|a| a.slug == slug)
}

pub async fn get_area_monitoring(
    pool: &AppState,
    area: &MonitoringArea,
) -> Result<dsr::AreaMonitoring> {
    let mut client = pool.pool_raport.get().await?;

    let equipment_ids: Vec<Vec<u8>> = area
        .equipment_ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .map(|u| u.into_bytes().to_vec())
        .collect();

    if equipment_ids.is_empty() {
        return Ok(dsr::AreaMonitoring {
            Equipment: Vec::new(),
            LastEmergency: None,
        });
    }

    let placeholders: Vec<String> = (1..=equipment_ids.len())
        .map(|i| format!("@P{}", i))
        .collect();

    let equipment_sql = format!(
        "
        SELECT
            e.Id,
            e.Name,
            dt.StartDateTime,
            dtt.Name AS DowntimeTypeName,
            CAST(COALESCE(c.IsEmergency, 0) AS BIT) AS IsEmergency
        FROM DTEquipments e
        LEFT JOIN Downtimes dt
            ON dt.EquipmentID = e.Id
            AND dt.EndDateTime IS NULL
        LEFT JOIN DowntimesTypes dtt
            ON dtt.Id = dt.DowntimeTypeID
        LEFT JOIN DowntimesTypesClassification c
            ON c.DowntimeTypeID = dt.DowntimeTypeID
        WHERE e.Id IN ({})
          AND e.Marked = 0
        ",
        placeholders.join(", ")
    );

    let mut equipment_query = Query::new(equipment_sql);
    for id in &equipment_ids {
        equipment_query.bind(id.clone());
    }

    let mut equipment = Vec::new();

    {
        let mut stream = equipment_query.query(&mut client).await?;

        while let Some(item) = stream.try_next().await? {
            if let QueryItem::Row(row) = item {
                let equipment_id = row
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| Uuid::from_slice(bytes).ok().map(|u| u.to_string()));

                let equipment_name = row.get::<&str, _>("Name").map(String::from);
                let start_date_time = row.get::<chrono::NaiveDateTime, _>("StartDateTime")
                    .map(|dt| dt.to_string());
                let downtime_type_name = row.get::<&str, _>("DowntimeTypeName").map(String::from);
                let is_emergency = row.get::<bool, _>("IsEmergency").unwrap_or(false);

                let status = if start_date_time.is_none() {
                    "run"
                } else if is_emergency {
                    "stop"
                } else {
                    "wait"
                };

                equipment.push(dsr::EquipmentStatus {
                    EquipmentID: equipment_id,
                    EquipmentName: equipment_name,
                    Status: Some(status.to_string()),
                    DowntimeTypeName: downtime_type_name,
                    StartDateTime: start_date_time,
                });
            }
        }
    }

    // Порядок в equipment_ids задаёт и фильтр, и сортировку одновременно —
    // строки из БД возвращаются в произвольном порядке, поэтому сортируем
    // их здесь по позиции в исходном списке.
    let order = area.equipment_ids;
    equipment.sort_by_key(|e| {
        e.EquipmentID
            .as_deref()
            .and_then(|id| order.iter().position(|o| o.eq_ignore_ascii_case(id)))
            .unwrap_or(order.len())
    });

    let last_emergency_sql = format!(
        "
        SELECT TOP 1
            e.Name AS EquipmentName,
            dtt.Name AS DowntimeTypeName,
            dt.Comment,
            dt.StartDateTime,
            dt.EndDateTime
        FROM Downtimes dt
        JOIN DTEquipments e ON e.Id = dt.EquipmentID
        JOIN DowntimesTypesClassification c ON c.DowntimeTypeID = dt.DowntimeTypeID
        LEFT JOIN DowntimesTypes dtt ON dtt.Id = dt.DowntimeTypeID
        WHERE dt.EquipmentID IN ({})
          AND c.IsEmergency = 1
        ORDER BY dt.StartDateTime DESC
        ",
        placeholders.join(", ")
    );

    let mut last_emergency_query = Query::new(last_emergency_sql);
    for id in &equipment_ids {
        last_emergency_query.bind(id.clone());
    }

    let mut last_emergency = None;

    {
        let mut stream = last_emergency_query.query(&mut client).await?;

        while let Some(item) = stream.try_next().await? {
            if let QueryItem::Row(row) = item {
                last_emergency = Some(dsr::LastEmergencyDowntime {
                    EquipmentName: row.get::<&str, _>("EquipmentName").map(String::from),
                    DowntimeTypeName: row.get::<&str, _>("DowntimeTypeName").map(String::from),
                    Comment: row.get::<&str, _>("Comment").map(String::from),
                    StartDateTime: row
                        .get::<chrono::NaiveDateTime, _>("StartDateTime")
                        .map(|dt| dt.to_string()),
                    EndDateTime: row
                        .get::<chrono::NaiveDateTime, _>("EndDateTime")
                        .map(|dt| dt.to_string()),
                });
            }
        }
    }

    Ok(dsr::AreaMonitoring {
        Equipment: equipment,
        LastEmergency: last_emergency,
    })
}

// ЛОКАЛЬНЫЙ РЕЖИМ МОНИТОРИНГ =============================================================================
pub async fn get_area_monitoring_local(
    pool: &AppState,
    area: &MonitoringArea,
) -> Result<dsr::AreaMonitoring> {
    let mut client = pool.pool_raport.get().await?;

    let equipment_ids: Vec<Vec<u8>> = area
        .equipment_ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .map(|u| u.into_bytes().to_vec())
        .collect();

    if equipment_ids.is_empty() {
        return Ok(dsr::AreaMonitoring {
            Equipment: Vec::new(),
            LastEmergency: None,
        });
    }

    let placeholders: Vec<String> = (1..=equipment_ids.len())
        .map(|i| format!("@P{}", i))
        .collect();

    let equipment_sql = format!(
        "
        SELECT
            e.Id,
            e.Name,
            dt.StartDateTime,
            dtt.Name AS DowntimeTypeName,
            CAST(COALESCE(c.IsEmergency, 0) AS BIT) AS IsEmergency
        FROM DTEquipments e
        LEFT JOIN Downtimes dt
            ON dt.EquipmentID = e.Id
            AND dt.EndDateTime IS NULL
        LEFT JOIN DowntimesTypes dtt
            ON dtt.Id = dt.DowntimeTypeID
        LEFT JOIN DowntimesTypesClassification c
            ON c.DowntimeTypeID = dt.DowntimeTypeID
        WHERE e.Id IN ({})
          AND e.Marked = 0
        ",
        placeholders.join(", ")
    );

    let mut equipment_query = Query::new(equipment_sql);
    for id in &equipment_ids {
        equipment_query.bind(id.clone());
    }

    let mut equipment = Vec::new();

    {
        let mut stream = equipment_query.query(&mut client).await?;

        while let Some(item) = stream.try_next().await? {
            if let QueryItem::Row(row) = item {
                let equipment_id = row
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| Uuid::from_slice(bytes).ok().map(|u| u.to_string()));

                let equipment_name = row.get::<&str, _>("Name").map(String::from);
                let start_date_time = row.get::<chrono::NaiveDateTime, _>("StartDateTime")
                    .map(|dt| dt.to_string());
                let downtime_type_name = row.get::<&str, _>("DowntimeTypeName").map(String::from);
                let is_emergency = row.get::<bool, _>("IsEmergency").unwrap_or(false);

                let status = if start_date_time.is_none() {
                    "run"
                } else if is_emergency {
                    "stop"
                } else {
                    "wait"
                };

                equipment.push(dsr::EquipmentStatus {
                    EquipmentID: equipment_id,
                    EquipmentName: equipment_name,
                    Status: Some(status.to_string()),
                    DowntimeTypeName: downtime_type_name,
                    StartDateTime: start_date_time,
                });
            }
        }
    }

    // Сортируем по порядку в equipment_ids
    let order = area.equipment_ids;
    equipment.sort_by_key(|e| {
        e.EquipmentID
            .as_deref()
            .and_then(|id| order.iter().position(|o| o.eq_ignore_ascii_case(id)))
            .unwrap_or(order.len())
    });

    // Получаем последнюю аварийную остановку из локальной БД
    let last_emergency_sql = format!(
        "
        SELECT TOP 1
            e.Name AS EquipmentName,
            dtt.Name AS DowntimeTypeName,
            dt.Comment,
            dt.StartDateTime,
            dt.EndDateTime
        FROM Downtimes dt
        JOIN DTEquipments e ON e.Id = dt.EquipmentID
        JOIN DowntimesTypesClassification c ON c.DowntimeTypeID = dt.DowntimeTypeID
        LEFT JOIN DowntimesTypes dtt ON dtt.Id = dt.DowntimeTypeID
        WHERE dt.EquipmentID IN ({})
          AND c.IsEmergency = 1
        ORDER BY dt.StartDateTime DESC
        ",
        placeholders.join(", ")
    );

    let mut last_emergency_query = Query::new(last_emergency_sql);
    for id in &equipment_ids {
        last_emergency_query.bind(id.clone());
    }

    let mut last_emergency = None;

    {
        let mut stream = last_emergency_query.query(&mut client).await?;

        while let Some(item) = stream.try_next().await? {
            if let QueryItem::Row(row) = item {
                last_emergency = Some(dsr::LastEmergencyDowntime {
                    EquipmentName: row.get::<&str, _>("EquipmentName").map(String::from),
                    DowntimeTypeName: row.get::<&str, _>("DowntimeTypeName").map(String::from),
                    Comment: row.get::<&str, _>("Comment").map(String::from),
                    StartDateTime: row
                        .get::<chrono::NaiveDateTime, _>("StartDateTime")
                        .map(|dt| dt.to_string()),
                    EndDateTime: row
                        .get::<chrono::NaiveDateTime, _>("EndDateTime")
                        .map(|dt| dt.to_string()),
                });
            }
        }
    }

    Ok(dsr::AreaMonitoring {
        Equipment: equipment,
        LastEmergency: last_emergency,
    })
}

// _______________________________________________________________________________________________________
pub async fn get_monitoring_downtimes_local(
    pool: &AppState,
    area: &MonitoringArea,
) -> Result<Vec<dsr::DowntimesRefactor>> {
    let mut client = pool.pool_raport.get().await?;

    let equipment_ids: Vec<Vec<u8>> = area
        .equipment_ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .map(|u| u.into_bytes().to_vec())
        .collect();

    if equipment_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = (1..=equipment_ids.len())
        .map(|i| format!("@P{}", i))
        .collect();

    let query = format!(
        "
        SELECT TOP 50
            d.Id as EAMID,
            d.EAMDocName,
            e.Name as EquipmentName,
            dep.Name as DepartmentName,
            d.StartDateTime,
            d.EndDateTime,
            dt.Name as DowntimeTypeName,
            d.Author,
            d.Comment
        FROM Downtimes d
        JOIN DTEquipments e ON d.EquipmentID = e.Id
        LEFT JOIN DTDepartments dep ON d.DepartmentID = dep.Id
        LEFT JOIN DowntimesTypes dt ON d.DowntimeTypeID = dt.Id
        WHERE d.EquipmentID IN ({})
        ORDER BY d.StartDateTime DESC
        ",
        placeholders.join(", ")
    );

    let mut query = Query::new(query);
    for id in &equipment_ids {
        query.bind(id.clone());
    }

    let mut result = Vec::new();
    let mut stream = query.query(&mut client).await?;

    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            let mut data = dsr::DowntimesRefactor::default();
            
            if let Some(bytes) = row.get::<&[u8], _>("EAMID") {
                if let Ok(uuid) = Uuid::from_slice(bytes) {
                    data.EAMID = Some(uuid.to_string());
                }
            }
            
            data.EAMDocName = row.get("EAMDocName").map(|s: &str| s.to_string());
            data.EquipmentName = row.get("EquipmentName").map(|s: &str| s.to_string());
            data.DepartmentName = row.get("DepartmentName").map(|s: &str| s.to_string());
            
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("StartDateTime") {
                data.StartDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            if let Some(dt) = row.get::<chrono::NaiveDateTime, _>("EndDateTime") {
                data.EndDateTime = Some(dt.format("%Y%m%d%H%M%S").to_string());
            }
            
            if let (Some(start), Some(end)) = (
                row.get::<chrono::NaiveDateTime, _>("StartDateTime"),
                row.get::<chrono::NaiveDateTime, _>("EndDateTime")
            ) {
                let duration = end - start;
                let hours = duration.num_hours();
                let minutes = duration.num_minutes() % 60;
                data.Duration = Some(format!("{}ч {}м", hours, minutes));
            }
            
            data.DowntimeTypeName = row.get("DowntimeTypeName").map(|s: &str| s.to_string());
            data.Author = row.get("Author").map(|s: &str| s.to_string());
            data.Comment = row.get("Comment").map(|s: &str| s.to_string());
            data.Deleted = Some(0);
            data.EAMDowntime = None;

            result.push(data);
        }
    }

    Ok(result)
}


// prev local
/*pub async fn update_insert(pool: &AppState, body: Value, result_http: Value) -> Result<()> {
    let mut client = pool.pool_raport.get().await?;

    // =========================================================
    // В ЛОКАЛЬНОМ РЕЖИМЕ ОТКЛЮЧАЕМ FOREIGN KEY
    // =========================================================
    let is_local_mode = body.get("_local_mode").and_then(|v| v.as_bool()).unwrap_or(false);
    
    if is_local_mode {
        // Отключаем FOREIGN KEY
        let disable_query = "ALTER TABLE Downtimes NOCHECK CONSTRAINT FK_Downtimes_Equip";
        if let Err(e) = client.execute(disable_query, &[]).await {
            eprintln!("⚠️ Не удалось отключить FOREIGN KEY: {}", e);
        } else {
            // println!("🔓 FOREIGN KEY отключена для локального сохранения");
        }
    }

    let eam_version = result_http
        .get("EAMVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bin_id = result_http
        .get("EAMID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

    let eam_doc = result_http
        .get("EAMDocName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Парсим даты
    let start_date_str = body
        .get("StartDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let dt = if !start_date_str.is_empty() {
        match NaiveDateTime::parse_from_str(start_date_str, "%Y%m%d%H%M%S") {
            Ok(dt) => dt,
            Err(e) => {
                eprintln!("❌ Ошибка парсинга StartDateTime '{}': {}", start_date_str, e);
                chrono::Local::now().naive_local()
            }
        }
    } else {
        chrono::Local::now().naive_local()
    };

    let dt1 = body
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S").ok());

    // Получаем EquipmentID
    let equipment_id = body
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let equipment_id_bin = if !equipment_id.is_empty() {
        match Uuid::parse_str(equipment_id) {
            Ok(uuid) => uuid.into_bytes().to_vec(),
            Err(e) => {
                eprintln!("❌ Ошибка парсинга EquipmentID '{}': {}", equipment_id, e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Получаем DepartmentID
    let department_id = body
        .get("DepartmentID")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let downtime_id_bin = body
        .get("DowntimeTypeID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

    let is_empty = body
        .get("EAMID")
        .and_then(|v| v.as_str())
        .map_or(true, |s| s.is_empty());

    let department_id_bin: Option<Vec<u8>> = if !department_id.is_empty() {
        match Uuid::parse_str(department_id) {
            Ok(uuid) => Some(uuid.into_bytes().to_vec()),
            Err(_) => None
        }
    } else {
        None
    };

    let author = body
        .get("Author")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let comment = body
        .get("Comment")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let result = if is_empty {
        let insert = "
            INSERT INTO Downtimes 
            (Id, EquipmentID, DepartmentID, DowntimeTypeID, StartDateTime, EndDateTime, Author, Comment, EAMDocName, EAMVersion)
            VALUES(@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9, @P10)
        ";
        
        client
            .execute(
                insert,
                &[
                    &bin_id,
                    &equipment_id_bin,
                    &department_id_bin,
                    &downtime_id_bin,
                    &dt,
                    &dt1,
                    &author,
                    &comment,
                    &eam_doc,
                    &eam_version,
                ],
            )
            .await
    } else {
        let update = "
            UPDATE Downtimes
            SET
                EquipmentID = @P1,
                DepartmentID = COALESCE(@P2, DepartmentID),
                DowntimeTypeID = @P3,
                StartDateTime = @P4,
                EndDateTime = @P5,
                Author = @P6,
                Comment = @P7
            WHERE Id = @P8
        ";

        client
            .execute(
                update,
                &[
                    &equipment_id_bin,
                    &department_id_bin,
                    &downtime_id_bin,
                    &dt,
                    &dt1,
                    &author,
                    &comment,
                    &bin_id,
                ],
            )
            .await
    };

    // Включаем FOREIGN KEY обратно
    if is_local_mode {
        let enable_query = "ALTER TABLE Downtimes CHECK CONSTRAINT FK_Downtimes_Equip";
        if let Err(e) = client.execute(enable_query, &[]).await {
            eprintln!("⚠️ Не удалось включить FOREIGN KEY: {}", e);
        } else {
            // println!("🔒 FOREIGN KEY включена обратно");
        }
    }

    match result {
        Ok(_) => {
            // println!("✅ Запись успешно сохранена в БД");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Ошибка вставки/обновления в БД: {}", e);
            Err(anyhow::anyhow!("Ошибка БД: {}", e))
        }
    }
}*/