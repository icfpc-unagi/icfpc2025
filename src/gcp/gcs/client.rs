use anyhow::{Context, Result, bail};
use reqwest::Url;

use crate::gcp::gcs::types::{FileInfo, ListResponse, ObjectItem};
use crate::gcp::get_access_token;

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

    loop {
        let mut url = Url::parse(&format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o",
            bucket
        ))?;
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("delimiter", "/");
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

        for p in body.prefixes {
            // Each prefix is eff_prefix + subdir/
            let name = p.strip_prefix(&eff_prefix).unwrap_or(&p).to_string();
            dirs.push(name);
        }
        for it in body.items {
            // Each item name is eff_prefix + file
            let rel = it
                .name
                .strip_prefix(&eff_prefix)
                .unwrap_or(&it.name)
                .to_string();
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

pub async fn list_dir(bucket: &str, prefix: &str) -> Result<(Vec<String>, Vec<String>)> {
    list_dir_internal(bucket, prefix, |_it, rel| Some(rel.to_string())).await
}

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

pub async fn get_object_metadata(bucket: &str, object: &str) -> Result<ObjectItem> {
    let token = get_access_token()
        .await
        .context("Failed to get access token")?;

    let client = reqwest::Client::new();
    let mut page_token: Option<String> = None;
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
    async fn list_dir_smoke() -> Result<()> {
        // Read-only listing on public-ish bucket path; expect no error
        let _ = list_dir("icfpc2025-data", "").await?;
        Ok(())
    }

    #[tokio::test]
    async fn get_sa_metadata_by_env() -> Result<()> {
        let password = env::var("UNAGI_PASSWORD").expect("UNAGI_PASSWORD not set");
        let object = format!("{}/service_account.json", password);
        let meta = get_object_metadata("icfpc2025-data", &object).await?;
        assert_eq!(meta.name, object);
        Ok(())
    }
}
