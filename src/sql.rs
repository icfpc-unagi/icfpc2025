// --- generic insert_with ---
pub fn insert_with<A>(query: &str, args: A) -> Result<u64>
where
    for<'q> A: sqlx::IntoArguments<'q, MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::insert_with(&CLIENT, query, args))
}
use anyhow::Result;
use once_cell::sync::Lazy;
use sqlx::mysql::{MySqlPoolOptions, MySqlRow};
use sqlx::{MySql, Pool};
use std::env;

use crate::sql_async;

// A shared tokio runtime to run async sqlx code from a sync context.
static TOKIO_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
});

static CLIENT: Lazy<Pool<MySql>> = Lazy::new(|| {
    let password = env::var("UNAGI_PASSWORD").unwrap_or_else(|_| "".into());
    let url = match env::var("MYSQL_SOCKET") {
        Ok(socket) => format!("mysql://root:{password}@localhost:3306/unagi?socket={socket}"),
        Err(_) => format!(
            "mysql://root:{password}@{hostname}:3306/unagi",
            hostname = env::var("MYSQL_HOSTNAME")
                .as_deref()
                .unwrap_or("104.198.121.248")
        ),
    };
    TOKIO_RUNTIME
        .block_on(MySqlPoolOptions::new().max_connections(10).connect(&url))
        .expect("Failed to create MySQL pool")
});

// --- select ---
pub fn select(query: &str) -> Result<Vec<MySqlRow>> {
    TOKIO_RUNTIME.block_on(sql_async::select(&CLIENT, query))
}

pub fn select1<T1>(query: &str, p1: T1) -> Result<Vec<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::select1(&CLIENT, query, p1))
}

pub fn select2<T1, T2>(query: &str, p1: T1, p2: T2) -> Result<Vec<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
    T2: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::select2(&CLIENT, query, p1, p2))
}

// --- row ---
pub fn row(query: &str) -> Result<Option<MySqlRow>> {
    TOKIO_RUNTIME.block_on(sql_async::row(&CLIENT, query))
}

pub fn row1<T1>(query: &str, p1: T1) -> Result<Option<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::row1(&CLIENT, query, p1))
}

pub fn row2<T1, T2>(query: &str, p1: T1, p2: T2) -> Result<Option<MySqlRow>>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
    T2: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::row2(&CLIENT, query, p1, p2))
}

// --- cell ---
pub fn cell1<T, T1>(query: &str, p1: T1) -> Result<Option<T>>
where
    T: for<'r> sqlx::Decode<'r, MySql> + sqlx::Type<MySql> + Send + Unpin,
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::cell1(&CLIENT, query, p1))
}

// --- exec ---
pub fn exec1<T1>(query: &str, p1: T1) -> Result<()>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::exec1(&CLIENT, query, p1))
}

// --- insert ---
pub fn insert1<T1>(query: &str, p1: T1) -> Result<u64>
where
    T1: for<'a> sqlx::Encode<'a, MySql> + sqlx::Type<MySql> + Send,
{
    TOKIO_RUNTIME.block_on(sql_async::insert1(&CLIENT, query, p1))
}

// --- exec_batch ---
pub fn exec_batch<I, P>(query: &str, params_iter: I) -> Result<()>
where
    I: IntoIterator<Item = P>,
    for<'a> P: 'a + Send + sqlx::IntoArguments<'a, MySql>,
{
    TOKIO_RUNTIME.block_on(sql_async::exec_batch(&CLIENT, query, params_iter))
}
