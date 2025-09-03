use std::collections::HashMap;

use crate::gce::types::*;

pub fn create_default_instance_request(name: &str) -> InstanceRequest {
    let mut labels = HashMap::new();
    labels.insert(
        "goog-ops-agent-policy".to_string(),
        "v2-x86-template-1-4-0".to_string(),
    );
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
                disk_type: "projects/icfpc-primary/zones/asia-northeast1-c/diskTypes/pd-balanced"
                    .to_string(),
                labels: disk_labels,
                source_image:
                    "projects/ubuntu-os-cloud/global/images/ubuntu-2404-noble-amd64-v20250828"
                        .to_string(),
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
        machine_type: "projects/icfpc-primary/zones/asia-northeast1-b/machineTypes/c2d-standard-4"
            .to_string(),
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
            subnetwork: "projects/icfpc-primary/regions/asia-northeast1/subnetworks/default"
                .to_string(),
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
        tags: Tags { items: vec![] },
        zone: "projects/icfpc-primary/zones/asia-northeast1-b".to_string(),
    }
}
