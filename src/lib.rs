use anyhow::Context;
#[cfg(feature = "reqwest")]
use reqwest::Client;

#[cfg(feature = "tokio")]
#[cfg(feature = "reqwest")]
pub mod www;

#[cfg(feature = "mysql")]
pub mod sql;

#[cfg(feature = "mysql")]
pub mod lock;

#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gcp;

pub trait SetMinMax {
    fn setmin(&mut self, v: Self) -> bool;
    fn setmax(&mut self, v: Self) -> bool;
}
impl<T> SetMinMax for T
where
    T: PartialOrd,
{
    fn setmin(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }
    fn setmax(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

#[macro_export]
macro_rules! mat {
    ($($e:expr),*) => { vec![$($e),*] };
    ($($e:expr,)*) => { vec![$($e),*] };
    ($e:expr; $d:expr) => { vec![$e; $d] };
    ($e:expr; $d:expr $(; $ds:expr)+) => { vec![mat![$e $(; $ds)*]; $d] };
}

#[cfg(feature = "reqwest")]
pub async fn get_bearer_async() -> anyhow::Result<String> {
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = Client::new();
    let res = client
        .get(format!(
            "https://storage.googleapis.com/icfpc2025-data/{}/bearer.txt",
            unagi_password,
        ))
        .send()
        .await
        .context("Failed to get bearer")?;
    res.text()
        .await
        .context("Failed to get bearer")
        .map(|s| format!("Bearer {}", s))
}

#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub fn get_bearer() -> anyhow::Result<String> {
    tokio::runtime::Runtime::new()?.block_on(get_bearer_async())
}

#[cfg(test)]
mod tests {}

#[cfg(feature = "reqwest")]
pub mod api;

pub mod problems;

pub mod judge;

pub mod mapgen {
    pub mod random;
}
