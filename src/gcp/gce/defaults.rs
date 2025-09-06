//! # GCE Instance Default Configurations
//!
//! This module provides helper functions to construct `InstanceRequest` objects
//! with sensible, project-specific default values. This simplifies the process
//! of creating new GCE VM instances by pre-filling many of the required fields.

use std::collections::HashMap;

use crate::gcp::gce::types::*;

/// Creates an `InstanceRequest` with a set of hardcoded default values.
///
/// This function is useful for creating a standard instance type with minimal input.
/// Most configuration, like machine type and disk image, is fixed.
///
/// # Arguments
/// * `name` - The name for the new instance.
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
                // Pinned to a specific Ubuntu 24.04 image version for reproducibility.
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
            // Use SPOT VMs for cost savings. They can be preempted.
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

/// Creates a more configurable `InstanceRequest`.
///
/// This function allows specifying key parameters like project, zone, machine type,
/// and an optional startup script, while still providing sensible defaults for
/// other fields.
///
/// # Arguments
/// * `name` - The name for the new instance.
/// * `project_id` - The GCP project ID.
/// * `zone` - The GCP zone for the instance (e.g., "us-central1-a").
/// * `machine_type` - The machine type (e.g., "e2-medium").
/// * `startup_script` - An optional shell script to run on instance startup.
pub fn create_instance_request(
    name: &str,
    project_id: &str,
    zone: &str,
    machine_type: &str,
    startup_script: Option<&str>,
) -> InstanceRequest {
    // Infer the region from the zone.
    let region = zone
        .rsplit_once('-')
        .map(|(prefix, _)| prefix)
        .unwrap_or(zone);

    let mut labels = HashMap::new();
    labels.insert(
        "goog-ops-agent-policy".to_string(),
        "v2-x86-template-1-4-0".to_string(),
    );
    labels.insert("goog-ec-src".to_string(), "vm_add-rest".to_string());

    let disk_labels = HashMap::new();

    let mut metadata_items = vec![MetadataItem {
        key: "enable-osconfig".to_string(),
        value: "TRUE".to_string(),
    }];

    // If a startup script is provided, add it to the instance metadata.
    if let Some(script) = startup_script {
        metadata_items.push(MetadataItem {
            key: "startup-script".to_string(),
            value: script.to_string(),
        });
    }

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
                disk_type: format!(
                    "projects/{}/zones/{}/diskTypes/pd-balanced",
                    project_id, zone
                ),
                labels: disk_labels,
                // Pinned to a specific Ubuntu 24.04 image version for reproducibility.
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
        machine_type: format!(
            "projects/{}/zones/{}/machineTypes/{}",
            project_id, zone, machine_type
        ),
        metadata: Metadata {
            items: metadata_items,
        },
        name: name.to_string(),
        network_interfaces: vec![NetworkInterface {
            access_configs: vec![AccessConfig {
                name: "External NAT".to_string(),
                network_tier: "PREMIUM".to_string(),
            }],
            stack_type: "IPV4_ONLY".to_string(),
            subnetwork: format!(
                "projects/{}/regions/{}/subnetworks/default",
                project_id, region
            ),
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
            // Use SPOT VMs for cost savings. They can be preempted.
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
        zone: format!("projects/{}/zones/{}", project_id, zone),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_paths() {
        let name = "test-vm";
        let project = "icfpc-primary";
        let zone = "asia-northeast1-b";
        let mtype = "c2d-standard-4";
        let req = create_instance_request(name, project, zone, mtype, None);
        assert!(
            req.machine_type
                .ends_with(&format!("/machineTypes/{}", mtype))
        );
        assert!(req.machine_type.contains(project));
        assert!(req.machine_type.contains(zone));
        assert!(req.zone.ends_with(&format!("/zones/{}", zone)));
        assert!(req.zone.contains(project));
        assert_eq!(req.name, name);
        assert_eq!(req.disks.len(), 1);
    }
}
