pub mod client;
pub mod defaults;
pub mod types;

// Re-export to preserve external API under icfpc2025::gcp::gce
pub use crate::gcp::gce::client::create_instance;
pub use crate::gcp::gce::defaults::{create_default_instance_request, create_instance_request};
pub use crate::gcp::gce::types::*;
