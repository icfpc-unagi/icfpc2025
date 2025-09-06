#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod auth;

#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gcs;

#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gce;

pub mod types;

// Re-export common auth
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub use auth::get_access_token;
