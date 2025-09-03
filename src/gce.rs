use anyhow::Result;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

const GCE_API_BASE: &str = "https://compute.googleapis.com/compute/v1";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "type")]
    pub account_type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

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

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

pub async fn get_access_token() -> Result<String> {
    let service_account_json = fs::read_to_string("secrets/service_account.json")?;
    let service_account: ServiceAccount = serde_json::from_str(&service_account_json)?;
    
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let exp = now + 3600;
    
    let claims = Claims {
        iss: service_account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: TOKEN_URL.to_string(),
        exp,
        iat: now,
    };
    
    let header = Header::new(Algorithm::RS256);
    let encoding_key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())?;
    
    let jwt = encode(&header, &claims, &encoding_key)?;
    
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", jwt.as_str()),
    ];
    
    let response = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Failed to get access token: {}", error_text));
    }
    
    let token_response: AccessToken = response.json().await?;
    Ok(token_response.access_token)
}

pub async fn create_instance(project_id: &str, zone: &str, instance_request: &InstanceRequest) -> Result<Value> {
    let token = get_access_token().await?;
    
    let client = reqwest::Client::new();
    let url = format!("{}/projects/{}/zones/{}/instances", GCE_API_BASE, project_id, zone);
    
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(instance_request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Failed to create instance: {}", error_text));
    }
    
    let result: Value = response.json().await?;
    Ok(result)
}

pub fn create_default_instance_request(name: &str) -> InstanceRequest {
    let mut labels = HashMap::new();
    labels.insert("goog-ops-agent-policy".to_string(), "v2-x86-template-1-4-0".to_string());
    labels.insert("goog-ec-src".to_string(), "vm_add-rest".to_string());
    
    let disk_labels = HashMap::new();
    
    InstanceRequest {
        can_ip_forward: false,
        confidential_instance_config: ConfidentialInstanceConfig {
            enable_confidential_compute: false,
        },
        deletion_protection: false,
        description: String::new(),
        disks: vec![Disk {
            auto_delete: true,
            boot: true,
            device_name: name.to_string(),
            disk_encryption_key: serde_json::json!({}),
            initialize_params: InitializeParams {
                disk_size_gb: "50".to_string(),
                disk_type: "projects/icfpc-primary/zones/asia-northeast1-c/diskTypes/pd-balanced".to_string(),
                labels: disk_labels,
                source_image: "projects/ubuntu-os-cloud/global/images/ubuntu-2404-noble-amd64-v20250828".to_string(),
            },
            mode: "READ_WRITE".to_string(),
            disk_type: "PERSISTENT".to_string(),
        }],
        display_device: DisplayDevice {
            enable_display: false,
        },
        guest_accelerators: vec![],
        instance_encryption_key: serde_json::json!({}),
        key_revocation_action_type: "NONE".to_string(),
        labels,
        machine_type: "projects/icfpc-primary/zones/asia-northeast1-b/machineTypes/c2d-standard-4".to_string(),
        metadata: Metadata {
            items: vec![MetadataItem {
                key: "enable-osconfig".to_string(),
                value: "TRUE".to_string(),
            }],
        },
        name: name.to_string(),
        network_interfaces: vec![NetworkInterface {
            access_configs: vec![AccessConfig {
                name: "External NAT".to_string(),
                network_tier: "PREMIUM".to_string(),
            }],
            stack_type: "IPV4_ONLY".to_string(),
            subnetwork: "projects/icfpc-primary/regions/asia-northeast1/subnetworks/default".to_string(),
        }],
        params: Params {
            resource_manager_tags: serde_json::json!({}),
        },
        reservation_affinity: ReservationAffinity {
            consume_reservation_type: "NO_RESERVATION".to_string(),
        },
        scheduling: Scheduling {
            automatic_restart: false,
            instance_termination_action: "STOP".to_string(),
            on_host_maintenance: "TERMINATE".to_string(),
            provisioning_model: "SPOT".to_string(),
        },
        service_accounts: vec![ServiceAccountRef {
            email: "289881194472-compute@developer.gserviceaccount.com".to_string(),
            scopes: vec!["https://www.googleapis.com/auth/cloud-platform".to_string()],
        }],
        shielded_instance_config: ShieldedInstanceConfig {
            enable_integrity_monitoring: true,
            enable_secure_boot: false,
            enable_vtpm: true,
        },
        tags: Tags {
            items: vec![],
        },
        zone: "projects/icfpc-primary/zones/asia-northeast1-b".to_string(),
    }
}