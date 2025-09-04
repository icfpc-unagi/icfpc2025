use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ListResponse {
    #[serde(default)]
    pub items: Vec<ObjectItem>,
    #[serde(default)]
    pub prefixes: Vec<String>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ObjectItem {
    pub name: String,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(rename = "contentType")]
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(rename = "storageClass")]
    #[serde(default)]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub crc32c: Option<String>,
    #[serde(rename = "md5Hash")]
    #[serde(default)]
    pub md5_hash: Option<String>,
    #[serde(default)]
    pub generation: Option<String>,
    #[serde(default)]
    pub metageneration: Option<String>,
    #[serde(default)]
    pub etag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: Option<u64>,
    pub updated: Option<String>,
}
