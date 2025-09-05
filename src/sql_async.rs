use anyhow::Result;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, Pool};

// --- select ---
pub async fn select(pool: &Pool<MySql>, query: &str) -> Result<Vec<MySqlRow>> {
    sqlx::query(query)
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn select1<T1>(pool: &Pool<MySql>, query: &str, p1: T1) -> Result<Vec<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query(query)
        .bind(p1)
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn select2<T1, T2>(
    pool: &Pool<MySql>,
    query: &str,
    p1: T1,
    p2: T2,
) -> Result<Vec<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
    T2: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query(query)
        .bind(p1)
        .bind(p2)
        .fetch_all(pool)
        .await
        .map_err(|e| e.into())
}

// --- row ---
pub async fn row(pool: &Pool<MySql>, query: &str) -> Result<Option<MySqlRow>> {
    sqlx::query(query)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn row1<T1>(pool: &Pool<MySql>, query: &str, p1: T1) -> Result<Option<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query(query)
        .bind(p1)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.into())
}

pub async fn row2<T1, T2>(
    pool: &Pool<MySql>,
    query: &str,
    p1: T1,
    p2: T2,
) -> Result<Option<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
    T2: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query(query)
        .bind(p1)
        .bind(p2)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.into())
}

// --- cell ---
pub async fn cell1<T, T1>(pool: &Pool<MySql>, query: &str, p1: T1) -> Result<Option<T>>
where
    T: for<'r> sqlx::Decode<'r, MySql> + sqlx::Type<MySql> + Send + Unpin,
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query_scalar(query)
        .bind(p1)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.into())
}

// --- exec ---
pub async fn exec1<T1>(pool: &Pool<MySql>, query: &str, p1: T1) -> Result<()>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    sqlx::query(query).bind(p1).execute(pool).await?;
    Ok(())
}

// --- insert ---
pub async fn insert1<T1>(pool: &Pool<MySql>, query: &str, p1: T1) -> Result<u64>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    let result = sqlx::query(query).bind(p1).execute(pool).await?;
    Ok(result.last_insert_id())
}

// --- exec_batch ---
pub async fn exec_batch<I, P>(pool: &Pool<MySql>, query: &str, params_iter: I) -> Result<()>
where
    I: IntoIterator<Item = P>,
    for<'a> P: 'a + Send + sqlx::IntoArguments<'a, MySql>,
{
    let mut tx = pool.begin().await?;
    for params in params_iter {
        sqlx::query_with(query, params).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
