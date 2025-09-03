pub mod auth;
pub mod client;
pub mod defaults;
pub mod types;

// Re-export commonly used items to preserve the existing public API.
pub use crate::gce::auth::get_access_token;
pub use crate::gce::client::create_instance;
pub use crate::gce::defaults::{create_default_instance_request, create_instance_request};
pub use crate::gce::types::*;
