use crate::structures::struct_pool_client::AppState;
use anyhow::Result;
use futures_util::TryStreamExt;
use tiberius::{Query, QueryItem};
// type DbPool = Pool<ConnectionManager>;

pub async fn dtypes(pool: &AppState, id: &str) -> Result<Option<String>> {
    let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DowntimesTypes where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    Ok(name)
}

pub async fn depart(pool: &AppState, id: &str) -> Result<Option<String>> {
    let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DTDepartments where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    Ok(name)
}

pub async fn equip(pool: &AppState, id: String) -> Result<Option<String>> {
    let mut client = pool.pool_raport.get().await?;
    let mut name = None;

    let clean_id = id.replace("-", "");

    let id_hex = hex::decode(&clean_id).unwrap_or_default();

    let mut select = Query::new("select Name from DTEquipments where Id = @P1");
    select.bind(id_hex);

    {
        let mut stream = select.query(&mut client).await?;
        while let Some(row) = stream.try_next().await? {
            if let QueryItem::Row(r) = row {
                name = r.get(0).map(|s: &str| s.to_string())
            }
        }
    }

    Ok(name)
}
