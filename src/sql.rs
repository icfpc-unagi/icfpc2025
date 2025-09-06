//! # MySQL Database Wrapper
//!
//! This module provides a simplified, opinionated interface for interacting with a
//! MySQL database. It uses a globally shared, lazily-initialized connection pool
//! (`CLIENT`) to manage database connections.
//!
//! The main goals are to reduce boilerplate for common query patterns and to provide
//! more ergonomic error handling and data access.
//!
//! ## Configuration
//!
//! The database connection is configured via environment variables:
//! - `UNAGI_PASSWORD`: The password for the `root` user.
//! - `MYSQL_SOCKET`: (Optional) Path to the MySQL socket file.
//! - `MYSQL_HOSTNAME`: (Optional) Hostname or IP of the MySQL server. Defaults to a hardcoded IP.

use anyhow::Result;
use mysql;
use mysql::prelude::*;
use mysql::*;
use once_cell::sync::Lazy;
use std::env;

/// A global, lazily-initialized MySQL connection pool.
///
/// The connection URL is constructed at first use, based on environment variables.
/// This allows the application to connect to the database without needing to
/// explicitly pass connection objects around.
static CLIENT: Lazy<mysql::Pool> = Lazy::new(|| {
    let password = env::var("UNAGI_PASSWORD").unwrap_or_else(|_| "".into());
    // The connection logic prioritizes a local socket if MYSQL_SOCKET is set,
    // otherwise it connects via TCP to a specified or default hostname.
    let url = match env::var("MYSQL_SOCKET") {
        Ok(socket) => format!(
            "mysql://root:{}@localhost:3306/unagi?socket={}",
            password, socket
        ),
        Err(_) => format!(
            "mysql://root:{}@{}:3306/unagi",
            password,
            env::var("MYSQL_HOSTNAME")
                .as_deref()
                .unwrap_or("104.198.121.248")
        ),
    };
    let opts = Opts::from_url(&url).expect("Invalid MySQL URL");
    let pool = Pool::new(opts).expect("Failed to create MySQL pool");
    eprintln!("MySQL connection established.");
    pool
});

/// Executes a query that is expected to return multiple rows.
///
/// # Returns
/// A `Result` containing a `Vec<Row>` of all rows returned by the query.
pub fn select(query: &str, params: impl Into<Params>) -> Result<Vec<Row>> {
    let mut conn = CLIENT.get_conn()?;
    conn.exec_map(query, params, |r| Row { row: r })
        .map_err(|e| e.into())
}

/// Executes a query that is expected to return at most one row.
///
/// # Returns
/// A `Result` containing an `Option<Row>`. `Some` if a row was found, `None` otherwise.
pub fn row(query: &str, params: impl Into<Params>) -> Result<Option<Row>> {
    Ok(CLIENT
        .get_conn()?
        .exec_first(query, params)?
        .map(|r| Row { row: r }))
}

/// Executes a query that is expected to return a single cell (one row, one column).
///
/// # Returns
/// A `Result` containing an `Option<T>`, where `T` is the type of the value in the cell.
/// `Some` if a row was found, `None` otherwise.
pub fn cell<T: FromValue>(query: &str, params: impl Into<Params>) -> Result<Option<T>> {
    match row(query, params)? {
        Some(row) => Ok(Some(row.at(0)?)),
        None => Ok(None),
    }
}

/// Executes a statement that does not return rows (e.g., UPDATE, DELETE, DDL).
///
/// # Returns
/// A `Result` containing the number of affected rows.
pub fn exec(query: &str, params: impl Into<Params>) -> Result<u64> {
    let mut conn = CLIENT.get_conn()?;
    conn.exec_drop(query, params)?;
    Ok(conn.affected_rows())
}

/// Executes an INSERT statement.
///
/// This is a convenience wrapper around `exec`.
///
/// # Returns
/// A `Result` containing the last insert ID.
pub fn insert(query: &str, params: impl Into<Params>) -> Result<u64> {
    let mut conn = CLIENT.get_conn()?;
    conn.exec_drop(query, params)?;
    Ok(conn.last_insert_id())
}

/// Executes a statement multiple times with different parameters in a single batch.
/// This is more efficient than executing the same statement repeatedly.
pub fn exec_batch<P, I>(query: &str, params: I) -> Result<()>
where
    P: Into<Params>,
    I: IntoIterator<Item = P>,
{
    let mut conn = CLIENT.get_conn()?;
    conn.exec_batch(query, params)?;
    Ok(())
}

/// A wrapper around `mysql::Row` that provides more ergonomic data access methods.
pub struct Row {
    row: mysql::Row,
}

impl Row {
    /// Gets an optional value from the row by column index.
    ///
    /// This handles the case where the database value is `NULL`.
    ///
    /// # Returns
    /// `Ok(Some(T))` if the value is not NULL.
    /// `Ok(None)` if the value is NULL.
    /// `Err` if the value cannot be converted to type `T`.
    pub fn at_option<T>(&self, idx: usize) -> Result<Option<T>>
    where
        T: FromValue,
    {
        match self.row.get_opt::<mysql::Value, usize>(idx) {
            Some(Ok(mysql::Value::NULL)) => None,
            Some(Ok(x)) => Some(mysql::from_value_opt::<T>(x.clone())),
            Some(Err(e)) => Some(Err(e)),
            None => None, // Should not happen if index is valid, but handle gracefully.
        }
        .transpose()
        .map_err(|e| {
            anyhow::anyhow!(
                "Error in column {} (#{}): {}",
                self.row.columns_ref()[idx].name_str(),
                idx,
                e
            )
        })
    }

    /// Gets a required value from the row by column index.
    ///
    /// # Returns
    /// `Ok(T)` if the value is not NULL and can be converted.
    /// `Err` if the value is NULL or cannot be converted.
    pub fn at<T>(&self, idx: usize) -> Result<T>
    where
        T: FromValue,
    {
        self.at_option(idx)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Column {} (#{}) is unexpectedly null",
                self.row.columns_ref()[idx].name_str(),
                idx
            )
        })
    }

    /// Finds the index of a column by its name.
    fn idx(&self, name: &str) -> Result<usize> {
        self.row
            .columns()
            .iter()
            .position(|c| c.name_str() == name)
            .ok_or_else(|| anyhow::anyhow!("Column {} is not found", name))
    }

    /// Gets a required value from the row by column name.
    pub fn get<T>(&self, name: &str) -> Result<T>
    where
        T: FromValue,
    {
        self.at(self.idx(name)?)
    }

    /// Gets an optional value from the row by column name.
    pub fn get_option<T>(&self, name: &str) -> Result<Option<T>>
    where
        T: FromValue,
    {
        self.at_option(self.idx(name)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysql::params;

    #[test]
    #[ignore]
    fn cell_select_literal() -> Result<()> {
        // 簡単なリテラル選択が取得できること
        let v: Option<i64> = cell("SELECT 1", ())?;
        assert_eq!(v, Some(1));
        Ok(())
    }

    #[test]
    #[ignore]
    fn row_and_named_access() -> Result<()> {
        let r = row("SELECT 42 AS a, NULL AS b", ())?.expect("row should exist");
        let a: i64 = r.get("a")?;
        let b: Option<i64> = r.get_option("b")?;
        assert_eq!(a, 42);
        assert_eq!(b, None);
        // 位置指定も動くこと
        let a0: i64 = r.at(0)?;
        assert_eq!(a0, 42);
        Ok(())
    }

    #[test]
    #[ignore]
    fn exec_insert_and_batch_with_temporary_table() -> Result<()> {
        // 同一コネクションで TEMPORARY TABLE を作成し、テスト内で完結させる
        let mut conn = CLIENT.get_conn()?;

        conn.exec_drop(
            "CREATE TEMPORARY TABLE tmp_agents (
                id INT AUTO_INCREMENT PRIMARY KEY,
                v INT
            )",
            (),
        )?;

        // insert（last_insert_id が返る）
        conn.exec_drop("INSERT INTO tmp_agents(v) VALUES (123)", ())?;
        assert_eq!(conn.last_insert_id(), 1);

        // batch で複数行追加
        conn.exec_batch(
            "INSERT INTO tmp_agents(v) VALUES (:v)",
            vec![params! {"v" => 456}, params! {"v" => 789}],
        )?;

        // 件数確認（同一コネクションなので TEMPORARY TABLE が見える）
        let cnt: Option<i64> = conn.exec_first("SELECT COUNT(*) FROM tmp_agents", ())?;
        assert_eq!(cnt, Some(3));

        // TEMPORARY TABLE はコネクションクローズで自動削除される
        Ok(())
    }
}
