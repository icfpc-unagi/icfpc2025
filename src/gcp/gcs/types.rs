//! # Google Cloud Storage (GCS) Data Types
//!
//! This module defines the Rust structs that model the JSON objects used in the
//! Google Cloud Storage API, particularly for listing and describing objects.

use serde::Deserialize;

/// Represents the response from a GCS `objects.list` API call.
#[derive(Debug, Deserialize, Default)]
pub struct ListResponse {
    /// A list of objects in the bucket that match the query.
    #[serde(default)]
    pub items: Vec<ObjectItem>,
    /// A list of common prefixes. This is used to emulate directories.
    #[serde(default)]
    pub prefixes: Vec<String>,
    /// A token that can be used to fetch the next page of results.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

/// Represents the metadata for a single GCS object.
///
/// This struct corresponds to the `Object` resource in the GCS JSON API.
/// Many fields are optional as their presence depends on the request.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ObjectItem {
    /// The name of the object.
    pub name: String,
    /// The name of the bucket containing this object.
    #[serde(default)]
    pub bucket: Option<String>,
    /// The size of the object in bytes.
    #[serde(default)]
    pub size: Option<String>,
    /// The last modification time of the object's metadata.
    #[serde(default)]
    pub updated: Option<String>,
    /// The content type of the object.
    #[serde(rename = "contentType")]
    #[serde(default)]
    pub content_type: Option<String>,
    /// The storage class of the object.
    #[serde(rename = "storageClass")]
    #[serde(default)]
    pub storage_class: Option<String>,
    /// The CRC32C checksum of the object's content.
    #[serde(default)]
    pub crc32c: Option<String>,
    /// The MD5 hash of the object's content.
    #[serde(rename = "md5Hash")]
    #[serde(default)]
    pub md5_hash: Option<String>,
    /// The generation number of the object's content.
    #[serde(default)]
    pub generation: Option<String>,
    /// The metageneration number of the object's metadata.
    #[serde(default)]
    pub metageneration: Option<String>,
    /// The HTTP ETag of the object.
    #[serde(default)]
    pub etag: Option<String>,
}

/// A simplified representation of an object in GCS, for user-facing functions.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// The full name (path) of the object.
    pub name: String,
    /// The size of the object in bytes.
    pub size: Option<u64>,
    /// The last modification time as a string.
    pub updated: Option<String>,
}
