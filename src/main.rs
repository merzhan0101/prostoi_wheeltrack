use actix_files as fs;
use actix_web::{App, HttpServer, web};
use bb8::Pool;
use bb8_tiberius::ConnectionManager;
use dotenv::dotenv;
use prostoi_wheeltrack::infra;
use prostoi_wheeltrack::structures::struct_pool_client::{AppState, DbPool};
use std::env;

async fn create_pool(env_var_name: &str) -> std::io::Result<DbPool> {
    let conn_str = env::var(env_var_name).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Missing env var: {}", e),
        )
    })?;

    let config = tiberius::Config::from_ado_string(&conn_str)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let manager = ConnectionManager::new(config);

    let pool = Pool::builder()
        .max_size(5)
        .build(manager)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e))?;

    Ok(pool)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let pool_wt = create_pool("CONNECTION_STRING").await?;
    let pool_raport = create_pool("CONNECTION_RAPORT").await?;

    let app_state = AppState {
        pool_wt,
        pool_raport,
    };

    let data_state = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(data_state.clone())
            .configure(infra::downtime::conf_workpiece_downtime)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
