//! # Google Cloud Storage (GCS) Client
//!
//! This module provides a client for interacting with the Google Cloud Storage API.
//! It is used for object-based storage operations like uploading, downloading,
//! and listing objects in GCS buckets.
//!
//! ## Submodules
//! - `client`: Contains the core client logic for making API requests to GCS.
//! - `types`: Defines the data structures that are serialized to and deserialized from
//!   the GCS API.

/// Core client for GCS API requests.
pub mod client;
/// Data structures for the GCS API.
pub mod types;

// Re-export key components to provide a convenient public API for this module.
pub use client::{
    download_object, get_object_metadata, list_dir, list_dir_detailed, parse_gs_url, upload_object,
};
pub use types::*;
