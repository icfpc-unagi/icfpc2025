//! # Google Cloud Platform (GCP) Utilities
//!
//! This module provides clients and utilities for interacting with various
//! Google Cloud Platform services that are used by the project infrastructure.
//!
//! Most of the functionality in this module requires the `reqwest` and `tokio`
//! features to be enabled, as it relies on asynchronous HTTP requests.
//!
//! ## Submodules
//! - `auth`: Handles authentication with GCP, providing access tokens.
//! - `gce`: A client for Google Compute Engine, used for managing virtual machine instances.
//! - `gcs`: A client for Google Cloud Storage, used for object storage.
//! - `types`: Contains common data types used across the GCP clients.

/// GCP authentication utilities.
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod auth;

/// A client for Google Cloud Storage (GCS).
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gcs;

/// A client for Google Compute Engine (GCE).
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub mod gce;

/// Common data types for GCP clients.
pub mod types;

// Re-export common auth functions for convenience.
#[cfg(all(feature = "reqwest", feature = "tokio"))]
pub use auth::get_access_token;
