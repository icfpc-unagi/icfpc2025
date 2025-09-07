use chrono::NaiveDateTime;
use icfpc2025::{problems::*, www::handlers::cron::insert_snapshot, *};

const BUCKET: &str = "icfpc2025-data";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (dirs, _files) = crate::gcp::gcs::list_dir(BUCKET, "history").await?;
    let stamps = dirs
        .into_iter()
        .inspect(|d| eprintln!("Found directory: {}", d))
        .map(|d| d.trim_end_matches('/').to_string());
    for ts_str in stamps {
        eprintln!("Processing timestamp {}... ", ts_str);
        // ts は "%Y%m%d-%H%M%S" 形式の文字列なのでパースして NaiveDateTime を得る
        let ts = NaiveDateTime::parse_from_str(&ts_str, "%Y%m%d-%H%M%S")
            .map_err(|e| anyhow::anyhow!("Failed to parse timestamp '{}': {}", ts_str, e))?;
        for Problem { problem, .. } in all_problems() {
            eprintln!("  Problem {}...", problem);
            let object = format!("history/{}/{}.json", ts_str, problem);
            match crate::gcp::gcs::download_object(BUCKET, &object).await {
                Ok(bytes) => {
                    eprintln!("  Downloaded object {} ({} bytes)", object, bytes.len());
                    let text = String::from_utf8(bytes).map_err(|e| {
                        anyhow::anyhow!("  Failed to decode object {}: {}", object, e)
                    })?;
                    insert_snapshot(&ts, problem, &text)?;
                    println!("  Inserted snapshot for {} {}", ts_str, problem);
                }
                Err(e) => {
                    eprintln!("  Error downloading object {}: {}", object, e);
                }
            }
        }
    }
    Ok(())
}
