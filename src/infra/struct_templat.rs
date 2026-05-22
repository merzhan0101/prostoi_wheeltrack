use askama::Template;

static BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Template)]
#[template(path = "downtimes.html")]
pub struct WorkpieceDowntimesTemplates {
    pub static_version: &'static str,
}

impl WorkpieceDowntimesTemplates {
    pub async fn render_page() -> Result<Self, Box<dyn std::error::Error>> {
        let static_version = BUILD_VERSION;
        Ok(Self { static_version })
    }
}

#[derive(Template)]
#[template(path = "dtpanel.html")]
pub struct WorkpieceRaportTemplates {
    pub static_version: &'static str,
}

impl WorkpieceRaportTemplates {
    pub async fn render_page() -> Result<Self, Box<dyn std::error::Error>> {
        let static_version = BUILD_VERSION;
        Ok(Self { static_version })
    }
}