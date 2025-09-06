//! # Google Compute Engine (GCE) Client
//!
//! This module provides a client for interacting with the Google Compute Engine API.
//! It is used for managing GCE virtual machine instances, such as creating new
//! instances based on templates or specific configurations.
//!
//! ## Submodules
//! - `client`: Contains the core client logic for making API requests to GCE.
//! - `defaults`: Provides helper functions to create default configurations for GCE instances.
//! - `types`: Defines the data structures that are serialized to and deserialized from
//!   the GCE API.

/// Core client for GCE API requests.
pub mod client;
/// Helper functions for creating default GCE instance configurations.
pub mod defaults;
/// Data structures for the GCE API.
pub mod types;

// Re-export key components to provide a convenient public API for this module.
pub use crate::gcp::gce::client::create_instance;
pub use crate::gcp::gce::defaults::{create_default_instance_request, create_instance_request};
pub use crate::gcp::gce::types::*;
