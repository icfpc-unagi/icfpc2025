use anyhow::{Context, Result, bail};

pub async fn run(url: &str) -> Result<()> {
    let (bucket, object) = icfpc2025::gcp::gcs::parse_gs_url(url)?;
    if object.is_empty() || object.ends_with('/') {
        bail!(
            "cat requires a full object path, not a bucket or prefix: {}",
            url
        );
    }
    let bytes = icfpc2025::gcp::gcs::download_object(&bucket, &object)
        .await
        .with_context(|| format!("Failed to download gs://{}/{}", bucket, object))?;
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    out.write_all(&bytes)?;
    Ok(())
}
