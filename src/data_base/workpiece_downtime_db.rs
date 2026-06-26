use crate::data_base::id_search as sh;
use crate::structures::workpiece_struct as wsr;
use anyhow::Result;
use chrono::NaiveDateTime;
use futures_util::TryStreamExt;
use serde_json::Value;
use tiberius::{Query, QueryItem};
use uuid::Uuid;

use crate::structures::struct_pool_client::AppState;

pub async fn get_name_depart(
    pool: &AppState,
    name_depart: Option<String>,
) -> Result<Vec<wsr::SearchNameDepartResult>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = wsr::SearchNameDepartResult::default();

    let like_query = format!("%{}%", name_depart.unwrap_or("".to_string()));

    let mut data_result: Vec<wsr::SearchNameDepartResult> = Vec::new();

    let mut select = Query::new("SELECT TOP 10 Id, Name FROM DTEquipments WHERE Name LIKE @P1");
    select.bind(like_query);

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

pub async fn get_name_worker(
    pool: &AppState,
    name_depart: Option<String>,
) -> Result<Vec<wsr::SearchNameWorkerResult>> {
    let mut client = pool.pool_wt.get().await?;

    let mut data = wsr::SearchNameWorkerResult::default();

    let like_query = format!("%{}%", name_depart.unwrap_or("".to_string()));

    let mut data_result: Vec<wsr::SearchNameWorkerResult> = Vec::new();

    let mut select = Query::new(
        "SELECT Id, FIO 
            FROM Worker 
            WHERE FIO LIKE @P1 and Place_id = 1 and Post_id = 1",
    );
    select.bind(like_query);

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                data.id = r.get("Id");
                data.fio = r.get("FIO").map(|s: &str| s.to_string());
                data_result.push(data.clone());
            }
        }
    }

    Ok(data_result)
}

pub async fn get_types(pool: &AppState) -> Result<Vec<wsr::DataTypes>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = wsr::DataTypes::default();

    let mut data_result: Vec<wsr::DataTypes> = Vec::new();

    let select = Query::new(
        "SELECT Id, Name FROM DowntimesTypes 
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
                data_result.push(data.clone());
            }
        }
    }

    Ok(data_result)
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

    // let department_id_bin = body
    //     .get("DepartmentID")
    //     .and_then(|v| v.as_str())
    //     .and_then(|s| Uuid::parse_str(s).ok())
    //     .map(|u| u.into_bytes().to_vec())
    //     .unwrap_or_default();

    // println!("BIN ID IS: {:#?}", bin_id);

    if is_empty {
        // let insert = "insert into Downtimes (Id, EquipmentID, DowntimeTypeID, StartDateTime, EndDateTime, Author, Comment, EAMDocName, EAMVersion) VALUES(@P1, @P2, @P3, @P4, @P5, @P6, @P7, @P8, @P9)";
        
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
        let update = "UPDATE Downtimes SET EquipmentID=@P1, DepartmentID=@P2, DowntimeTypeID=@P3, StartDateTime=@P4, EndDateTime=@P5, Author=@P6, Comment=@P7 WHERE Id=@P8";

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
) -> Result<Vec<wsr::DowntimesRefactor>> {
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

    let mut data = wsr::DowntimesRefactor::default();

    let mut data_send: Vec<wsr::DowntimesRefactor> = Vec::new();

    for item in data_array {
        data.EAMID = item.get("EAMID").and_then(|v| v.as_str()).map(String::from);

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
) -> Result<wsr::GetDowntime> {
    let mut client = pool.pool_raport.get().await?;

    let data = json.get("Data").unwrap();
    let mut data_toir = wsr::GetDowntime::default();

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

pub async fn data_update(pool: &AppState, body: Value) -> Result<wsr::UpdateData> {
    let mut client = pool.pool_raport.get().await?;
    let mut data = wsr::UpdateData::default();

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

    data.EAMVersion = body
        .get("EAMVersion")
        .and_then(|v| v.as_str())
        .map(String::from);
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

// test
// pub async fn get_name_department(
//     pool: &AppState,
//     name_depart: Option<String>,
// ) -> Result<Vec<wsr::DepartmentName>> {
//     let mut client = pool.pool_wt.get().await?;

//     let mut data = wsr::DepartmentName::default();

//     let like_query = format!("%{}%", name_depart.unwrap_or("".to_string()));

//     let mut data_result: Vec<wsr::DepartmentName> = Vec::new();

//     let mut select = Query::new(
//         "SELECT Id, Name FROM DTDepartments WHERE Marked = 0 and Code in ('000000196', '000000209', '000000122', '000000030')",
//     );
//     select.bind(like_query);

//     {
//         let mut stream = select.query(&mut client).await?;
//         while let Some(row) = stream.try_next().await? {
//             if let QueryItem::Row(r) = row {
//                 data.id = r.get("Id");
//                 data.department = r.get("Name").map(|s: &str| s.to_string());
//                 data_result.push(data.clone());
//             }
//         }
//     }

//     Ok(data_result)
// }

pub async fn get_name_department(
    pool: &AppState,
    name_depart: Option<String>,
) -> Result<Vec<wsr::DepartmentName>> {
    let mut client = pool.pool_raport.get().await?;

    let mut data = wsr::DepartmentName::default();

    let like_query = format!("%{}%", name_depart.unwrap_or("".to_string()));

    let mut data_result: Vec<wsr::DepartmentName> = Vec::new();

    let mut select = Query::new("SELECT Id, Name FROM DTDepartments WHERE Marked = 0 and Name in ('Участок по ремонту оборудования проката', 'Участок по ремонту гидравлического оборудования', 'Отдел АСУП колесного производства')");
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