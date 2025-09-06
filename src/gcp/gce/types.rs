//! # Google Compute Engine (GCE) Data Types
//!
//! This module defines the Rust structs that model the JSON objects used in the
//! Google Compute Engine API, specifically for creating new VM instances.
//! These structs are designed to be serialized into the JSON payload for an
//! `instances.insert` API request.
//!
//! For detailed information on each field, refer to the official GCE API documentation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents the request body for creating a new GCE virtual machine instance.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceRequest {
    /// Allows this instance to send and receive packets with non-matching destination or source IPs.
    #[serde(rename = "canIpForward")]
    pub can_ip_forward: bool,
    /// Settings for a confidential VM.
    #[serde(rename = "confidentialInstanceConfig")]
    pub confidential_instance_config: ConfidentialInstanceConfig,
    /// Whether the instance is protected against accidental deletion.
    #[serde(rename = "deletionProtection")]
    pub deletion_protection: bool,
    /// A brief description of the instance.
    pub description: String,
    /// The disks attached to the instance.
    pub disks: Vec<Disk>,
    /// Configuration for the instance's display device.
    #[serde(rename = "displayDevice")]
    pub display_device: DisplayDevice,
    /// A list of guest accelerator cards attached to the instance.
    #[serde(rename = "guestAccelerators")]
    pub guest_accelerators: Vec<Value>,
    /// Encryption key for the instance.
    #[serde(rename = "instanceEncryptionKey")]
    pub instance_encryption_key: Value,
    /// Action to take upon key a revocation.
    #[serde(rename = "keyRevocationActionType")]
    pub key_revocation_action_type: String,
    /// User-defined labels for the instance.
    pub labels: HashMap<String, String>,
    /// The machine type for this instance (e.g., "e2-medium").
    #[serde(rename = "machineType")]
    pub machine_type: String,
    /// Metadata key/value pairs available to the instance.
    pub metadata: Metadata,
    /// The name of the instance.
    pub name: String,
    /// The network interfaces for the instance.
    #[serde(rename = "networkInterfaces")]
    pub network_interfaces: Vec<NetworkInterface>,
    /// Additional parameters for the instance.
    pub params: Params,
    /// Specifies a reservation affinity for the instance.
    #[serde(rename = "reservationAffinity")]
    pub reservation_affinity: ReservationAffinity,
    /// Scheduling options for the instance.
    pub scheduling: Scheduling,
    /// The service accounts associated with the instance.
    #[serde(rename = "serviceAccounts")]
    pub service_accounts: Vec<ServiceAccountRef>,
    /// Configuration for Shielded VM features.
    #[serde(rename = "shieldedInstanceConfig")]
    pub shielded_instance_config: ShieldedInstanceConfig,
    /// A list of network tags for the instance.
    pub tags: Tags,
    /// The zone where the instance will be created (e.g., "us-central1-a").
    pub zone: String,
}

/// Confidential VM configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfidentialInstanceConfig {
    #[serde(rename = "enableConfidentialCompute")]
    pub enable_confidential_compute: bool,
}

/// An attached disk configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct Disk {
    #[serde(rename = "autoDelete")]
    pub auto_delete: bool,
    pub boot: bool,
    #[serde(rename = "deviceName")]
    pub device_name: String,
    #[serde(rename = "diskEncryptionKey")]
    pub disk_encryption_key: Value,
    #[serde(rename = "initializeParams")]
    pub initialize_params: InitializeParams,
    pub mode: String,
    #[serde(rename = "type")]
    pub disk_type: String,
}

/// Parameters for initializing a disk, typically from a source image.
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "diskSizeGb")]
    pub disk_size_gb: String,
    #[serde(rename = "diskType")]
    pub disk_type: String,
    pub labels: HashMap<String, String>,
    #[serde(rename = "sourceImage")]
    pub source_image: String,
}

/// Display device configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayDevice {
    #[serde(rename = "enableDisplay")]
    pub enable_display: bool,
}

/// Instance metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub items: Vec<MetadataItem>,
}

/// A single metadata key-value pair.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataItem {
    pub key: String,
    pub value: String,
}

/// A network interface for the instance.
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    #[serde(rename = "accessConfigs")]
    pub access_configs: Vec<AccessConfig>,
    #[serde(rename = "stackType")]
    pub stack_type: String,
    pub subnetwork: String,
}

/// Configuration for external network access.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessConfig {
    pub name: String,
    #[serde(rename = "networkTier")]
    pub network_tier: String,
}

/// Additional instance parameters.
#[derive(Debug, Serialize, Deserialize)]
pub struct Params {
    #[serde(rename = "resourceManagerTags")]
    pub resource_manager_tags: Value,
}

/// Reservation affinity settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReservationAffinity {
    #[serde(rename = "consumeReservationType")]
    pub consume_reservation_type: String,
}

/// Instance scheduling options.
#[derive(Debug, Serialize, Deserialize)]
pub struct Scheduling {
    #[serde(rename = "automaticRestart")]
    pub automatic_restart: bool,
    #[serde(rename = "instanceTerminationAction")]
    pub instance_termination_action: String,
    #[serde(rename = "onHostMaintenance")]
    pub on_host_maintenance: String,
    #[serde(rename = "provisioningModel")]
    pub provisioning_model: String,
}

/// A reference to a service account and its scopes.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccountRef {
    pub email: String,
    pub scopes: Vec<String>,
}

/// Shielded VM configuration.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShieldedInstanceConfig {
    #[serde(rename = "enableIntegrityMonitoring")]
    pub enable_integrity_monitoring: bool,
    #[serde(rename = "enableSecureBoot")]
    pub enable_secure_boot: bool,
    #[serde(rename = "enableVtpm")]
    pub enable_vtpm: bool,
}

/// A list of network tags.
#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub items: Vec<String>,
}
