#[cfg(feature = "reqwest")]
pub mod client;

// # Unagi: The Main Library for the ICFP 2025 Contest
//
// This crate contains all the core logic for the Unagi project, which is designed
// to solve the "The Name of the Binding" problem for ICFP 2025.
//
// The crate is highly modular and uses feature flags (`reqwest`, `tokio`, `mysql`)
// to enable different functionalities, such as API communication, database interaction,
// and web server capabilities.

use anyhow::Context;

/// WWW server implementation. Enabled with `tokio` and `reqwest` features.
#[cfg(feature = "tokio")]
#[cfg(feature = "reqwest")]
pub mod www;

/// SQL database interaction utilities. Enabled with the `mysql` feature.
#[cfg(feature = "mysql")]
pub mod sql;

/// Distributed lock management client. Enabled with the `mysql` feature.
#[cfg(feature = "mysql")]
pub mod lock;

/// Google Cloud Platform utilities. Enabled with `reqwest` and `tokio` features.
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gcp;

/// Task executor (DB-backed queue + runner). Requires MySQL for DB and reqwest+tokio for GCS uploads.
#[cfg(all(feature = "mysql", feature = "reqwest", feature = "tokio"))]
pub mod executor;
pub mod lock_guard;

/// A trait for conveniently updating a value to its minimum or maximum.
pub trait SetMinMax {
    /// If `v` is less than `self`, updates `self` to `v` and returns `true`.
    /// Otherwise, returns `false`.
    fn setmin(&mut self, v: Self) -> bool;
    /// If `v` is greater than `self`, updates `self` to `v` and returns `true`.
    /// Otherwise, returns `false`.
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

/// A macro for convenient initialization of vectors, including nested vectors for multi-dimensional arrays.
///
/// # Examples
///
/// ```
/// use icfpc2025::mat;
/// // A simple vector
/// let v1 = mat![1, 2, 3];
///
/// // A 2x3 matrix initialized with zeros
/// let m1 = mat![0; 2; 3];
/// assert_eq!(m1, vec![vec![0, 0, 0], vec![0, 0, 0]]);
/// ```
#[macro_export]
macro_rules! mat {
    ($($e:expr),*) => { vec![$($e),*] };
    ($($e:expr,)*) => { vec![$($e),*] };
    ($e:expr; $d:expr) => { vec![$e; $d] };
    ($e:expr; $d:expr $(; $ds:expr)+) => { vec![mat![$e $(; $ds)*]; $d] };
}

/// Asynchronously fetches the bearer token from Google Cloud Storage.
///
/// Requires the `UNAGI_PASSWORD` environment variable to be set.
/// This function is available when the `reqwest` feature is enabled.
#[cfg(feature = "reqwest")]
pub async fn get_bearer_async() -> anyhow::Result<String> {
    let unagi_password = std::env::var("UNAGI_PASSWORD").context("UNAGI_PASSWORD not set")?;
    let client = &*client::CLIENT;
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

/// Synchronously fetches the bearer token from Google Cloud Storage.
///
/// This is a blocking wrapper around `get_bearer_async`.
/// It is available when both `reqwest` and `tokio` features are enabled.
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub fn get_bearer() -> anyhow::Result<String> {
    tokio::runtime::Runtime::new()?.block_on(get_bearer_async())
}

#[cfg(test)]
mod tests {}

/// Client for the official contest web service (Aedificium).
/// Enabled with the `reqwest` feature.
#[cfg(feature = "reqwest")]
pub mod api;

/// Definitions and data for the contest problems.
pub mod problems;

/// Abstraction for the problem environment (the "Aedificium"), with local and remote implementations.
pub mod judge;

/// Utilities for generating SVG visualizations of maps.
pub mod svg;

/// Tools for generating problem maps.
pub mod mapgen {
    /// A module for generating random maps.
    pub mod random;
}

pub mod routes;

pub mod solve_no_marks;
