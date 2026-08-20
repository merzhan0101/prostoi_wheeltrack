use askama::Template;

static BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Template)]
#[template(path = "downtimes.html")]
pub struct DowntimesTemplate {
    pub static_version: &'static str,
    pub area_name: String,
    pub area_slug: String,
    pub area_department_id: String,
}

impl DowntimesTemplate {
    pub fn new(
        area_name: &str,
        area_slug: &str,
        area_department_id: &str,
    ) -> Self {
        Self {
            static_version: BUILD_VERSION,
            area_name: area_name.to_string(),
            area_slug: area_slug.to_string(),
            area_department_id: area_department_id.to_string(),
        }
    }
}

// МОНИТОРИНГ =========================================================================
#[derive(Template)]
#[template(path = "areas_monitoring.html")]
pub struct MonitoringTemplate {
    pub static_version: &'static str,
}

impl MonitoringTemplate {
    pub fn new() -> Self {
        Self {
            static_version: BUILD_VERSION,
        }
    }
}