use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceRequest {
    #[serde(rename = "canIpForward")]
    pub can_ip_forward: bool,
    #[serde(rename = "confidentialInstanceConfig")]
    pub confidential_instance_config: ConfidentialInstanceConfig,
    #[serde(rename = "deletionProtection")]
    pub deletion_protection: bool,
    pub description: String,
    pub disks: Vec<Disk>,
    #[serde(rename = "displayDevice")]
    pub display_device: DisplayDevice,
    #[serde(rename = "guestAccelerators")]
    pub guest_accelerators: Vec<Value>,
    #[serde(rename = "instanceEncryptionKey")]
    pub instance_encryption_key: Value,
    #[serde(rename = "keyRevocationActionType")]
    pub key_revocation_action_type: String,
    pub labels: HashMap<String, String>,
    #[serde(rename = "machineType")]
    pub machine_type: String,
    pub metadata: Metadata,
    pub name: String,
    #[serde(rename = "networkInterfaces")]
    pub network_interfaces: Vec<NetworkInterface>,
    pub params: Params,
    #[serde(rename = "reservationAffinity")]
    pub reservation_affinity: ReservationAffinity,
    pub scheduling: Scheduling,
    #[serde(rename = "serviceAccounts")]
    pub service_accounts: Vec<ServiceAccountRef>,
    #[serde(rename = "shieldedInstanceConfig")]
    pub shielded_instance_config: ShieldedInstanceConfig,
    pub tags: Tags,
    pub zone: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfidentialInstanceConfig {
    #[serde(rename = "enableConfidentialCompute")]
    pub enable_confidential_compute: bool,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayDevice {
    #[serde(rename = "enableDisplay")]
    pub enable_display: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub items: Vec<MetadataItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataItem {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    #[serde(rename = "accessConfigs")]
    pub access_configs: Vec<AccessConfig>,
    #[serde(rename = "stackType")]
    pub stack_type: String,
    pub subnetwork: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessConfig {
    pub name: String,
    #[serde(rename = "networkTier")]
    pub network_tier: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Params {
    #[serde(rename = "resourceManagerTags")]
    pub resource_manager_tags: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReservationAffinity {
    #[serde(rename = "consumeReservationType")]
    pub consume_reservation_type: String,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccountRef {
    pub email: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShieldedInstanceConfig {
    #[serde(rename = "enableIntegrityMonitoring")]
    pub enable_integrity_monitoring: bool,
    #[serde(rename = "enableSecureBoot")]
    pub enable_secure_boot: bool,
    #[serde(rename = "enableVtpm")]
    pub enable_vtpm: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tags {
    pub items: Vec<String>,
}
