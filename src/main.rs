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
            format!("Missing env var: {}", env_var_name)
            // format!("Missing env var: {}", e),
        )
    })?;

    let config = tiberius::Config::from_ado_string(&conn_str)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let manager = ConnectionManager::new(config);

    let pool = Pool::builder()
        .max_size(15) // Увеличиваем лимит соединений
        .connection_timeout(std::time::Duration::from_secs(45)) // Даем больше времени на ожидание
        .build(manager)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e))?;

    // println!("pool=> {:#?}", pool);
    Ok(pool)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // println!("dotenv = {:?}", dotenv());
    // println!("CONNECTION_STRING = {:?}", env::var("CONNECTION_STRING"));

    let toir_url = env::var("TOIR_URL").expect("TOIR_URL not found");
    let toir_login = env::var("TOIR_LOGIN").expect("TOIR_LOGIN not found");
    let toir_password = env::var("TOIR_PASSWORD").expect("TOIR_PASSWORD not found");

    let pool_wt = create_pool("CONNECTION_STRING").await?;
    let pool_raport = create_pool("CONNECTION_RAPORT").await?;

    let app_state = AppState {
        pool_wt,
        pool_raport,

        toir_url,
        toir_login,
        toir_password,
    };

    let data_state = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(data_state.clone())
            .configure(infra::downtime::conf_workpiece_downtime)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    })

    .bind("127.0.0.1:2000")?
    // .bind("0.0.0.0:8080")?
    .bind("10.110.7.253:8080")?
    .run()
    .await
}