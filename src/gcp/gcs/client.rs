//! # GCS API Client Logic
//!
//! This module contains the core client functions for making API requests to
//! Google Cloud Storage for operations like listing, downloading, and uploading objects.

use anyhow::{Context, Result, bail};
use reqwest::Url;

use crate::gcp::gcs::types::{FileInfo, ListResponse, ObjectItem};
use crate::gcp::get_access_token;

/// Parses a GCS URL string (`gs://bucket/object/path`) into a bucket and object prefix.
pub fn parse_gs_url(s: &str) -> Result<(String, String)> {
    let rest = s
        .strip_prefix("gs://")
        .context("URL must start with gs://")?;
    let (bucket, prefix) = match rest.split_once('/') {
        Some((b, p)) => (b.to_string(), p.to_string()),
        None => (rest.to_string(), String::new()),
    };
    if bucket.is_empty() {
        bail!("Bucket is empty in URL: {}", s);
    }
    Ok((bucket, prefix))
}

/// Internal generic function for listing objects in a GCS directory.
/// It handles pagination and separates results into "directories" (prefixes) and "files" (items).
/// The `map` function allows customizing the output format for file items.
async fn list_dir_internal<T, F>(
    bucket: &str,
    prefix: &str,
    map: F,
) -> Result<(Vec<String>, Vec<T>)>
where
    F: Fn(ObjectItem, &str) -> Option<T>,
{
    // For directory-like listing, ensure prefix ends with '/'
    let mut eff_prefix = prefix.to_string();
    if !eff_prefix.is_empty() && !eff_prefix.ends_with('/') {
        eff_prefix.push('/');
    }

    let token = get_access_token()
        .await
        .context("Failed to get access token")?;
    let client = reqwest::Client::new();

    let mut page_token: Option<String> = None;
    let mut dirs: Vec<String> = Vec::new();
    let mut out_items: Vec<T> = Vec::new();

    // Loop to handle paginated results from the GCS API.
    loop {
        let mut url = Url::parse(&format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o",
            bucket
        ))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("delimiter", "/"); // Use delimiter to get directory-like behavior.
            if !eff_prefix.is_empty() {
                qp.append_pair("prefix", &eff_prefix);
            }
            if let Some(ref t) = page_token {
                qp.append_pair("pageToken", t);
            }
        }

        let res = client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to call GCS list API")?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            bail!("GCS list failed ({}): {}", status, body);
        }

        let body: ListResponse = res.json().await.context("Invalid GCS response")?;

        // Prefixes are the "subdirectories".
        for p in body.prefixes {
            let name = p.strip_prefix(&eff_prefix).unwrap_or(&p).to_string();
            dirs.push(name);
        }
        // Items are the "files".
        for it in body.items {
            let rel = it
                .name
                .strip_prefix(&eff_prefix)
                .unwrap_or(&it.name)
                .to_string();
            // Skip the directory placeholder object itself.
            if rel.is_empty() || rel.ends_with('/') {
                continue;
            }
            if let Some(mapped) = map(it, &rel) {
                out_items.push(mapped);
            }
        }

        page_token = body.next_page_token;
        if page_token.is_none() {
            break;
        }
    }

    dirs.sort();
    Ok((dirs, out_items))
}

/// Lists the contents of a "directory" in a GCS bucket.
///
/// # Returns
/// A tuple containing two vectors:
/// 1. A list of subdirectory names.
/// 2. A list of file names.
pub async fn list_dir(bucket: &str, prefix: &str) -> Result<(Vec<String>, Vec<String>)> {
    list_dir_internal(bucket, prefix, |_it, rel| Some(rel.to_string())).await
}

/// Lists the contents of a "directory" in a GCS bucket with detailed file information.
///
/// # Returns
/// A tuple containing two vectors:
/// 1. A list of subdirectory names.
/// 2. A list of `FileInfo` structs for each file.
pub async fn list_dir_detailed(bucket: &str, prefix: &str) -> Result<(Vec<String>, Vec<FileInfo>)> {
    let (dirs, files) = list_dir_internal(bucket, prefix, |it, rel| {
        let size = it.size.as_deref().and_then(|s| s.parse::<u64>().ok());
        let updated = it.updated.clone();
        Some(FileInfo {
            name: rel.to_string(),
            size,
            updated,
        })
    })
    .await?;

    let mut files = files;
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((dirs, files))
}

/// Downloads an object from a GCS bucket.
///
/// # Returns
/// A `Vec<u8>` containing the raw bytes of the object.
pub async fn download_object(bucket: &str, object: &str) -> Result<Vec<u8>> {
    let token = get_access_token()
        .await
        .context("Failed to get access token")?;
    let client = reqwest::Client::new();

    // GCS API requires object paths to be percent-encoded as a single path segment.
    // This helper ensures characters like '/' are correctly encoded.
    fn encode_component(s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 3);
        for b in s.as_bytes() {
            let c = *b as char;
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                out.push(c);
            } else {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
        out
    }
    let encoded = encode_component(object);
    let url = Url::parse(&format!(
        "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
        bucket, encoded
    ))?;

    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context("Failed to download GCS object")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        bail!("GCS download failed ({}): {}", status, body);
    }

    let bytes = res.bytes().await.context("Failed to read GCS body")?;
    Ok(bytes.to_vec())
}

/// Uploads data as a new object to a GCS bucket.
///
/// # Arguments
/// * `bucket` - The destination bucket name.
/// * `name` - The full path and name for the new object.
/// * `data` - The raw byte data to upload.
/// * `content_type` - The MIME type of the data (e.g., "text/plain").
///
/// # Returns
/// An `ObjectItem` containing the metadata of the newly created object.
pub async fn upload_object(
    bucket: &str,
    name: &str,
    data: &[u8],
    content_type: &str,
) -> Result<ObjectItem> {
    let token = get_access_token()
        .await
        .context("Failed to get access token")?;
    let client = reqwest::Client::new();

    // Use the "media" upload type for simple, one-shot uploads.
    let mut url = Url::parse(&format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o",
        bucket
    ))?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("uploadType", "media");
        qp.append_pair("name", name);
    }

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", content_type)
        .body(data.to_vec())
        .send()
        .await
        .context("Failed to call GCS upload API")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        bail!("GCS upload failed ({}): {}", status, body);
    }

    let item: ObjectItem = res.json().await.context("Invalid GCS upload response")?;
    Ok(item)
}

/// Fetches the metadata for a single object in a GCS bucket.
///
/// This function uses the `list` API with a `prefix` filter to find the exact
/// object, as there is no direct "get metadata" endpoint that works with slashes
/// in the object name without special encoding.
pub async fn get_object_metadata(bucket: &str, object: &str) -> Result<ObjectItem> {
    let token = get_access_token()
        .await
        .context("Failed to get access token")?;

    let client = reqwest::Client::new();
    let mut page_token: Option<String> = None;
    // Loop to handle pagination, though for a unique object name, we expect one result.
    loop {
        let mut url = Url::parse(&format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o",
            bucket
        ))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("prefix", object);
            qp.append_pair(
                "fields",
                "items(name,size,updated,contentType,storageClass,crc32c,md5Hash,generation,metageneration,etag,bucket),nextPageToken",
            );
            if let Some(ref t) = page_token {
                qp.append_pair("pageToken", t);
            }
        }

        let res = client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .context("Failed to call GCS get object via list API")?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            bail!("GCS get object failed ({}): {}", status, body);
        }
        let body: ListResponse = res.json().await.context("Invalid GCS response")?;
        // Find the exact match from the list results.
        if let Some(item) = body.items.into_iter().find(|it| it.name == object) {
            return Ok(item);
        }
        page_token = body.next_page_token;
        if page_token.is_none() {
            break;
        }
    }
    bail!("Object not found: gs://{}/{}", bucket, object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::env;

    #[tokio::test]
    async fn parse_gs_url_basic() -> Result<()> {
        let (b, p) = parse_gs_url("gs://bucket").unwrap();
        assert_eq!(b, "bucket");
        assert_eq!(p, "");
        let (b, p) = parse_gs_url("gs://bucket/prefix/dir").unwrap();
        assert_eq!(b, "bucket");
        assert_eq!(p, "prefix/dir");
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn list_dir_smoke() -> Result<()> {
        // Read-only listing on public-ish bucket path; expect no error
        let _ = list_dir("icfpc2025-data", "").await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn get_sa_metadata_by_env() -> Result<()> {
        let password = env::var("UNAGI_PASSWORD").expect("UNAGI_PASSWORD not set");
        let object = format!("{}/service_account.json", password);
        let meta = get_object_metadata("icfpc2025-data", &object).await?;
        assert_eq!(meta.name, object);
        Ok(())
    }
}
