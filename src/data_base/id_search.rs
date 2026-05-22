use crate::structures::struct_pool_client::AppState;
use anyhow::Result;
use futures_util::TryStreamExt;
use tiberius::{Query, QueryItem};

use tokio::net::TcpStream;
use tokio_util::compat::Compat;

// type DbPool = Pool<ConnectionManager>;

pub async fn dtypes(
    client: &mut tiberius::Client<Compat<TcpStream>>,
    id: &str,
) -> Result<Option<String>> {
    // let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DowntimesTypes where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    Ok(name)
}

pub async fn depart(
    client: &mut tiberius::Client<Compat<TcpStream>>,
    id: &str,
) -> Result<Option<String>> {
    // let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DTDepartments where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    Ok(name)
}

pub async fn equip(
    client: &mut tiberius::Client<Compat<TcpStream>>,
    id: String,
) -> Result<Option<String>> {
    // let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DTEquipments where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    // let state = pool.pool_raport.state();
    // println!(
    //     "СТАТУС ПУЛА в функции equip: Всего соединений: {}, Открыто прямо сейчас: {}",
    //     state.connections,      // сколько физически открыто в БД
    //     state.idle_connections, // сколько из них СВОБОДНО прямо сейчас   // сколько потоков СЕЙЧАС ждут в очереди (если > 0, мы в дедлоке)
    // );

    Ok(name)
}