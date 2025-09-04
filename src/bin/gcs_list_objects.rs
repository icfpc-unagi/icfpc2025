use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "gcs_list_objects",
    about = "List GCS objects like ls for a gs:// URL"
)]
struct Args {
    /// Enable long listing format (size, updated)
    #[arg(short = 'l', long = "long")]
    long: bool,

    /// Recursive listing
    #[arg(short = 'R', long = "recursive")]
    recursive: bool,

    /// gs://bucket[/prefix or object]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let (bucket, prefix) = icfpc2025::gcs::parse_gs_url(&args.url)?;

    // If prefix is not empty and not ending with '/', try object metadata
    if !prefix.is_empty()
        && !prefix.ends_with('/')
        && let Ok(meta) = icfpc2025::gcs::get_object_metadata(&bucket, &prefix).await
    {
        print_object_details(&bucket, &meta)?;
        return Ok(());
    }

    if args.recursive {
        walk_recursive(&bucket, &prefix, &args).await?;
    } else if args.long {
        let (dirs, files) = icfpc2025::gcs::list_dir_detailed(&bucket, &prefix).await?;
        for d in dirs {
            print_long_dir(&d);
        }
        for f in files {
            print_long_file(&f);
        }
    } else {
        let (dirs, files) = icfpc2025::gcs::list_dir(&bucket, &prefix).await?;
        for d in dirs {
            println!("{}/", d.trim_end_matches('/'));
        }
        for f in files {
            println!("{}", f);
        }
    }
    Ok(())
}

fn print_long_dir(name: &str) {
    println!("{:>12}  {:<20}  {}/", "-", "-", name.trim_end_matches('/'));
}

fn print_long_file(f: &icfpc2025::gcs::types::FileInfo) {
    let size = f
        .size
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());
    let updated = f.updated.as_deref().unwrap_or("-");
    println!("{:>12}  {:<20}  {}", size, updated, f.name);
}

async fn walk_recursive(bucket: &str, prefix: &str, args: &Args) -> Result<()> {
    let mut stack: Vec<String> = vec![prefix.to_string()];
    while let Some(current) = stack.pop() {
        // Header
        let header = if current.is_empty() {
            format!("gs://{}/", bucket)
        } else {
            format!("gs://{}/{}", bucket, current)
        };
        println!("{}:", header);

        if args.long {
            let (mut dirs, files) = icfpc2025::gcs::list_dir_detailed(bucket, &current).await?;
            for d in &dirs {
                print_long_dir(d);
            }
            for f in &files {
                print_long_file(f);
            }
            println!();
            // Push children in reverse to process in lexical order
            dirs.sort();
            for d in dirs.into_iter().rev() {
                let new_prefix = if current.is_empty() {
                    d
                } else {
                    format!("{}/{}", current.trim_end_matches('/'), d)
                };
                stack.push(new_prefix);
            }
        } else {
            let (mut dirs, files) = icfpc2025::gcs::list_dir(bucket, &current).await?;
            for d in &dirs {
                println!("{}/", d.trim_end_matches('/'));
            }
            for f in &files {
                println!("{}", f);
            }
            println!();
            dirs.sort();
            for d in dirs.into_iter().rev() {
                let new_prefix = if current.is_empty() {
                    d
                } else {
                    format!("{}/{}", current.trim_end_matches('/'), d)
                };
                stack.push(new_prefix);
            }
        }
    }
    Ok(())
}

fn print_object_details(bucket: &str, meta: &icfpc2025::gcs::types::ObjectItem) -> Result<()> {
    let name = &meta.name;
    let size = meta.size.as_deref().unwrap_or("-");
    let updated = meta.updated.as_deref().unwrap_or("-");
    let content_type = meta.content_type.as_deref().unwrap_or("-");
    let storage_class = meta.storage_class.as_deref().unwrap_or("-");
    let crc32c = meta.crc32c.as_deref().unwrap_or("-");
    let md5 = meta.md5_hash.as_deref().unwrap_or("-");
    let generation_str = meta.generation.as_deref().unwrap_or("-");
    let metagen = meta.metageneration.as_deref().unwrap_or("-");
    let etag = meta.etag.as_deref().unwrap_or("-");

    println!("Name: gs://{}/{}", bucket, name);
    println!("Size: {}", size);
    println!("Updated: {}", updated);
    println!("Content-Type: {}", content_type);
    println!("Storage-Class: {}", storage_class);
    println!("CRC32C: {}", crc32c);
    println!("MD5: {}", md5);
    println!("Generation: {}", generation_str);
    println!("Metageneration: {}", metagen);
    println!("ETag: {}", etag);
    Ok(())
}
