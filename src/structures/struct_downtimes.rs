use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaPath {
    pub area: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchName {
    pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchNameDepartResult {
    pub name: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchNameWorkerResult {
    pub fio: Option<String>,
    pub id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepartmentName {
    pub department: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataTypes {
    pub id: Option<String>,
    pub name: Option<String>,
    pub is_emergency: Option<bool>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EquipmentStatus {
    pub EquipmentID: Option<String>,
    pub EquipmentName: Option<String>,
    pub Status: Option<String>, // "run" | "wait" | "stop"
    pub DowntimeTypeName: Option<String>,
    pub StartDateTime: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LastEmergencyDowntime {
    pub EquipmentName: Option<String>,
    pub DowntimeTypeName: Option<String>,
    pub Comment: Option<String>,
    pub StartDateTime: Option<String>,
    pub EndDateTime: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AreaMonitoring {
    pub Equipment: Vec<EquipmentStatus>,
    pub LastEmergency: Option<LastEmergencyDowntime>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DowntimesRefactor {
    pub EAMID: Option<String>,
    pub EAMDocName: Option<String>,
    pub EAMDowntime: Option<i32>,
    pub EquipmentName: Option<String>,
    pub DepartmentName: Option<String>,
    pub StartDateTime: Option<String>,
    pub EndDateTime: Option<String>,
    pub Duration: Option<String>,
    pub DowntimeTypeName: Option<String>,
    pub Author: Option<String>,
    pub Deleted: Option<i32>,
    pub Comment: Option<String>,    
}

// __________________________

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetDowntimesRequest {
    pub Filter_StartDate: Option<String>,
    pub Filter_EndDate: Option<String>,
    pub Filter_EquipmentID: Option<String>,
    pub Filter_DepartmentID: Option<String>,
    pub WithEAMDowntimes: Option<i32>,
    pub ShowDeleted: Option<i32>,
    pub OrderSettings: Option<String>,
    pub MaxRows: Option<i32>,
    pub Offset: Option<i32>,
    // pub Comment: Option<String>,
}

// _________________________________
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiResponse {
    pub errors: Option<serde_json::Value>,
    pub data: Vec<DowntimeItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DowntimeItem {
    pub eamid: String,
    pub eam_doc_name: String,
    pub eam_downtime: i32,
    pub equipment_id: String,
    pub department_id: String,
    pub start_date_time: String,
    pub end_date_time: String,
    pub downtime_type_id: String,
    pub duration: i64,
    pub author: String,
    pub deleted: i32,
    pub posted: i32,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetDowntime {
    pub EAMID: Option<String>,
    pub EAMDocName: Option<String>,
    pub EAMDowntime: Option<i32>,
    pub EquipmentID: Option<String>,
    pub EquipmentName: Option<String>,
    pub DepartmentID: Option<String>,
    pub StartDateTime: Option<String>,
    pub EndDateTime: Option<String>,
    pub Duration: Option<i32>,
    pub DowntimeTypeID: Option<String>,
    pub Comment: Option<String>,
    pub Author: Option<String>,
    pub Posted: Option<i32>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetPosted {
    pub Posted: Option<i32>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateData {
    pub EAMVersion: Option<String>,
    pub EAMID: Option<String>,
    pub EquipmentID: Option<String>,
    pub DepartmentID: Option<String>,
    pub DepartmentIDs: Option<Vec<String>>,  // массив служб
    pub StartDateTime: Option<String>,
    pub EndDateTime: Option<String>,
    pub DowntimeTypeID: Option<String>,
    pub Comment: Option<String>,
    pub Author: Option<String>,
    pub Area: Option<String>,  
}

// #[allow(non_snake_case)]
// #[derive(Debug, Clone, Serialize, Deserialize, Default)]
// pub struct UpdateData {
//     pub EAMVersion: Option<String>,
//     pub EAMID: Option<String>,
//     pub EquipmentID: Option<String>,
//     pub StartDateTime: Option<String>,
//     pub EndDateTime: Option<String>,
//     pub DowntimeTypeID: Option<String>,
//     pub Comment: Option<String>,
//     pub Author: Option<String>,
// }