use anyhow::Result;
use mysql;
use mysql::prelude::*;
use mysql::*;
use once_cell::sync::Lazy;
use std::env;

static CLIENT: Lazy<mysql::Pool> = Lazy::new(|| {
    let password = env::var("UNAGI_PASSWORD").unwrap_or_else(|_| "".into());
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
                .unwrap_or("34.84.6.97")
        ),
    };
    let pool = Pool::new(url).unwrap();
    eprintln!("MySQL connection established.");
    pool
});

pub fn select(query: &str, params: impl Into<Params>) -> Result<Vec<Row>> {
    let mut conn = CLIENT.get_conn()?;
    conn.exec_map(query, params, |r| Row { row: r })
        .map_err(|e| e.into())
}

pub fn row(query: &str, params: impl Into<Params>) -> Result<Option<Row>> {
    Ok(CLIENT
        .get_conn()?
        .exec_first(query, params)?
        .map(|r| Row { row: r }))
}

pub fn cell<T: FromValue>(query: &str, params: impl Into<Params>) -> Result<Option<T>> {
    match row(query, params)? {
        Some(row) => Ok(Some(row.at(0)?)),
        None => Ok(None),
    }
}

pub fn exec(query: &str, params: impl Into<Params>) -> Result<()> {
    CLIENT
        .get_conn()?
        .exec_drop(query, params)
        .map_err(|e| e.into())
}

// insert is the same as exec, but it returns the last insert ID.
pub fn insert(query: &str, params: impl Into<Params>) -> Result<u64> {
    let mut conn = CLIENT.get_conn()?;
    conn.exec_drop(query, params)?;
    Ok(conn.last_insert_id())
}

pub fn exec_batch<P, I>(query: &str, params: I) -> Result<()>
where
    P: Into<Params>,
    I: IntoIterator<Item = P>,
{
    let mut conn = CLIENT.get_conn()?;
    conn.exec_batch(query, params)?;
    Ok(())
}

pub struct Row {
    row: mysql::Row,
}

impl Row {
    pub fn at_option<T>(&self, idx: usize) -> Result<Option<T>>
    where
        T: FromValue,
    {
        match self.row.get_opt::<mysql::Value, usize>(idx) {
            Some(Ok(mysql::Value::NULL)) => None,
            Some(Ok(x)) => Some(mysql::from_value_opt::<T>(x.clone())),
            Some(Err(e)) => Some(Err(e)),
            None => None,
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

    fn idx(&self, name: &str) -> Result<usize> {
        name.idx(&self.row.columns())
            .ok_or_else(|| anyhow::anyhow!("Column {} is not found", name))
    }

    pub fn get<T>(&self, name: &str) -> Result<T>
    where
        T: FromValue,
    {
        self.at(self.idx(name)?)
    }

    pub fn get_option<T>(&self, name: &str) -> Result<Option<T>>
    where
        T: FromValue,
    {
        self.at_option(self.idx(name)?)
    }
}
