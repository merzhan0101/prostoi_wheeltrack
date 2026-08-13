use crate::data_base::id_search as sh;
use crate::structures::struct_downtimes as dsr;
use anyhow::Result;
use chrono::NaiveDateTime;
use futures_util::TryStreamExt;
use serde_json::Value;
use tiberius::{Query, QueryItem};
use uuid::Uuid;

use crate::structures::struct_pool_client::AppState;

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
        SELECT TOP 10
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

pub async fn get_types(pool: &AppState) -> Result<Vec<dsr::DataTypes>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = dsr::DataTypes::default();

    let mut data_result: Vec<dsr::DataTypes> = Vec::new();

    let select = Query::new("SELECT Id, Name FROM DowntimesTypes");

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.id = r
                    .get::<&[u8], _>("Id")
                    .and_then(|bytes| Uuid::from_slice(bytes).ok().map(|u| u.to_string()));
                data.name = r.get("Name").map(|s: &str| s.to_string());
                data_result.push(data.clone());
            }
        }
    }

    Ok(data_result)
}

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

pub async fn update_insert(pool: &AppState, body: Value, result_http: Value) -> Result<()> {
    let mut client = pool.pool_raport.get().await?;

    // println!("result_http: {:#?}", result_http);

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

    let dt = NaiveDateTime::parse_from_str(
        &body
            .get("StartDateTime")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "%Y%m%d%H%M%S",
    )
    .unwrap();

    let dt1 = body
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S").ok());

    // let bin_id = hex::decode(&eam_id).unwrap_or_default();

    let equipment_id_bin = body
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

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

    let department_id_bin: Option<Vec<u8>> = body
        .get("DepartmentID")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec());

    // println!("BIN ID IS: {:#?}", bin_id);

    if is_empty {

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
                    &body
                        .get("Author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    &body
                        .get("Comment")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                    &eam_doc,
                    &eam_version,
                ],
            )
            .await
            .unwrap();
    } else {
        // let update = "UPDATE Downtimes SET EquipmentID=@P1, DepartmentID=@P2, DowntimeTypeID=@P3, StartDateTime=@P4, EndDateTime=@P5, Author=@P6, Comment=@P7 WHERE Id=@P8";

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
                            &equipment_id_bin,      // @P1
                            &department_id_bin,     // @P2
                            &downtime_id_bin,       // @P3
                            &dt,                    // @P4
                            &dt1,                   // @P5
                            &body
                                .get("Author")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),       // @P6
                            &body
                                .get("Comment")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string()), // @P7
                            &bin_id,                // @P8
                        ]
            )
            .await
            .unwrap();
    }

    Ok(())
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

pub async fn data_update(pool: &AppState, body: Value) -> Result<dsr::UpdateData> {
    let mut client = pool.pool_raport.get().await?;
    let mut data = dsr::UpdateData::default();

    data.EAMID = body.get("EAMID").and_then(|v| v.as_str()).map(String::from);

    let bin_id = body
        .get("EAMID")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(|u| u.into_bytes().to_vec())
        .unwrap_or_default();

    let mut select_version = Query::new("SELECT TOP 1 EAMVersion FROM Downtimes WHERE Id=@P1");
    select_version.bind(bin_id);

    {
        let mut stream = select_version.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.EAMVersion = r.get(0).map(|s: &str| s.to_string());
            }
        }
    }

    /*data.EAMVersion = body
        .get("EAMVersion")
        .and_then(|v| v.as_str())
        .map(String::from);*/
    data.EquipmentID = body
        .get("EquipmentID")
        .and_then(|v| v.as_str())
        .map(String::from);
    data.DowntimeTypeID = body
        .get("DowntimeTypeID")
        .and_then(|v| v.as_str())
        .map(String::from);
    data.StartDateTime = body
        .get("StartDateTime")
        .and_then(|v| v.as_str())
        .map(String::from);
    data.EndDateTime = body
        .get("EndDateTime")
        .and_then(|v| v.as_str())
        .map(String::from);
    data.Author = body
        .get("Author")
        .and_then(|v| v.as_str())
        .map(String::from);
    data.Comment = body
        .get("Comment")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(data)
}

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
                    0xF26321C79D6C11ED816300155D16FE68
                );"
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