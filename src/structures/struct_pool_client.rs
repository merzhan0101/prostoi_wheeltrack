use bb8::Pool;
use bb8_tiberius::ConnectionManager;

pub type DbPool = Pool<ConnectionManager>;

#[derive(Clone, Debug)]
pub struct AppState {
    pub pool_wt: DbPool,
    pub pool_raport: DbPool,
}
