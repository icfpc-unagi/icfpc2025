//! # Web Server Implementation
//!
//! This module contains the implementation of the Unagi web server,
//! which is likely used for serving a dashboard, API, or other web-based
//! interfaces for the project.
//!
//! ## Submodules
//! - `handlers`: Contains the Axum request handlers for different API routes.
//! - `utils`: Provides utility functions used by the web server.

/// Request handlers for the web server's API routes.
pub mod handlers;
/// Utility functions for the web server.
pub mod utils;
