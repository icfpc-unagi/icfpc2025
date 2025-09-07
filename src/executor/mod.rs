use anyhow::{Context, Result};
use mysql::params;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::time::{Duration, Instant};

use crate::sql;
use std::path::Path;

pub mod lock;
pub mod run;

/// Information required to execute a task.
pub struct Task {
    pub task_id: i64,
    pub problem_name: String,
    pub problem_variant: i64,
    pub agent_name: String,
    pub agent_code: String,
    pub agent_bin: Option<String>,
    pub task_lock: String,
}

/// Attempts to acquire the next available task.
///
/// Algorithm:
/// - Pick the row with the oldest `task_locked` that is NOT NULL and not in the future.
/// - Set `task_locked` to now + 30s, set a new random `task_lock` token.
/// - If previous `task_lock` was NOT NULL, increment `task_failed` by 1.
/// - If the resulting `task_failed` is >= 3, set `task_locked` to NULL and give up the task.
pub fn acquire_task() -> Result<Option<Task>> {
    // 1) Generate new lock token
    let lock_token = gen_lock_token();

    eprintln!(
        "[executor] trying to acquire a task with lock={}",
        lock_token
    );

    // 2) Atomically update one candidate task
    let task_host = current_hostname();
    let affected = sql::exec(
        r#"
        UPDATE tasks t
        JOIN (
            SELECT task_id, task_lock
            FROM tasks
            WHERE task_locked IS NOT NULL
              AND task_locked <= CURRENT_TIMESTAMP
            ORDER BY task_locked ASC
            LIMIT 1
        ) sel ON t.task_id = sel.task_id
        SET
            t.task_failed = t.task_failed + IF(sel.task_lock IS NULL, 0, 1),
            t.task_lock = :task_lock,
            t.task_locked = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 SECOND),
            t.task_host = :task_host
        "#,
        params! { "task_lock" => &lock_token, "task_host" => &task_host },
    )?;

    if affected == 0 {
        eprintln!("[executor] no task acquired");
        return Ok(None);
    }

    // 3) Fetch the updated task row using the new lock token
    let row = match sql::row(
        r#"
        SELECT task_id, agent_id, problem_name, problem_variant, task_failed
        FROM tasks
        WHERE task_lock = :task_lock
          AND task_locked > CURRENT_TIMESTAMP
        "#,
        params! { "task_lock" => &lock_token },
    )? {
        Some(r) => r,
        None => return Ok(None),
    };

    let task_id: i64 = row.get("task_id")?;
    let agent_id: i64 = row.get("agent_id")?;
    let problem_name: String = row.get("problem_name")?;
    let problem_variant: i64 = row.get("problem_variant")?;
    let task_failed: i64 = row.get("task_failed")?;

    eprintln!(
        "[executor] candidate acquired: token={} (checking failures)",
        lock_token
    );

    // 4) If task_failed >= 3, release this task by clearing task_locked
    if task_failed >= 3 {
        eprintln!(
            "[executor] skipping task_id={} due to task_failed={} (clearing lock)",
            task_id, task_failed
        );
        let _ = sql::exec(
            r#"UPDATE tasks SET task_locked = NULL WHERE task_id = :task_id AND task_lock = :task_lock"#,
            params! { "task_id" => task_id, "task_lock" => &lock_token },
        )?;
        return Ok(None);
    }

    // 5) Join agents to get the code
    let row = sql::row(
        r#"
        SELECT t.task_id, t.problem_name, t.problem_variant, a.agent_name, a.agent_code, a.agent_bin
        FROM tasks t
        JOIN agents a ON a.agent_id = :agent_id
        WHERE t.task_id = :task_id
        "#,
        params! { "agent_id" => agent_id, "task_id" => task_id },
    )?
    .context("Agent for task not found")?;

    let agent_name: String = row.get("agent_name")?;
    let agent_code: String = row.get("agent_code")?;
    let agent_bin: Option<String> = row.get_option("agent_bin")?;

    eprintln!(
        "[executor] acquired task: id={} problem={} variant={} agent={}",
        task_id, problem_name, problem_variant, agent_name
    );

    Ok(Some(Task {
        task_id,
        problem_name,
        problem_variant,
        agent_name,
        agent_code,
        agent_bin,
        task_lock: lock_token,
    }))
}

/// Executes the agent code with placeholders substituted and captures logs.
///
/// - Substitutes {{problem_name}}, {{problem_variant}}, {{task_id}}, {{agent_name}}.
/// - Runs using `bash -lc` with a 600s timeout.
/// - Writes stdout/stderr as JSONL lines to `target/logs/{task_id}/stdout.jsonl` and `stderr.jsonl`.
/// - Uploads both files to `gs://icfpc2025-data/logs/{task_id}/`.
/// - Returns the parsed `score` from the last line starting with "<UNAGI>:" in stdout.
pub fn run_task(task: &Task) -> Result<(Option<i64>, i32, u128)> {
    // Prepare command by substituting placeholders
    let mut script = task.agent_code.clone();
    script = script.replace("\r", "");
    script = script.replace("{{problem_name}}", &task.problem_name);
    script = script.replace("{{problem_variant}}", &task.problem_variant.to_string());
    script = script.replace("{{task_id}}", &task.task_id.to_string());
    script = script.replace("{{agent_name}}", &task.agent_name);

    eprintln!("[executor] starting task_id={}", task.task_id);
    // Prepare cancel flag and heartbeat (lock management only)
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let hb_task_id = task.task_id;
    let hb_lock = task.task_lock.clone();
    let hb_stop = Arc::clone(&stop_flag);
    let hb_cancel = Arc::clone(&cancel);
    let _hb = std::thread::spawn(move || {
        let mut failed_count = 0usize;
        let mut next_extend = Instant::now() + Duration::from_secs(10);
        loop {
            if hb_stop.load(Ordering::Relaxed) {
                break;
            }
            if Instant::now() < next_extend {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            match crate::executor::lock::extend_lock(hb_task_id, &hb_lock) {
                Ok(true) => {
                    failed_count = 0;
                    next_extend = Instant::now() + Duration::from_secs(10);
                }
                Ok(false) => {
                    eprintln!(
                        "[executor] lock extend returned false for task_id={}, cancelling",
                        hb_task_id
                    );
                    hb_cancel.store(true, Ordering::Relaxed);
                    break;
                }
                Err(e) => {
                    failed_count += 1;
                    eprintln!(
                        "[executor] lock extend error (#{}) for task_id={}: {}",
                        failed_count, hb_task_id, e
                    );
                    if failed_count >= 3 {
                        eprintln!(
                            "[executor] lock extend failed {} times for task_id={}, cancelling",
                            failed_count, hb_task_id
                        );
                        hb_cancel.store(true, Ordering::Relaxed);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    });

    // Execute the script (execution only)
    let start = Instant::now();
    let (score, status, artifacts): (Option<i64>, std::process::ExitStatus, run::Artifacts) =
        match run::run_command_with_timeout(
            &script,
            Duration::from_secs(600),
            Arc::clone(&cancel),
            |arts| {
                if let Some(ref url) = task.agent_bin {
                    prepare_agent_bin(url, arts.root_dir())?;
                }
                Ok(())
            },
            &run::RunOptions::default(),
        ) {
            (Ok((s, st)), arts) => (s, st, arts),
            (Err(e), arts) => {
                eprintln!(
                    "[executor] run_command failed for task_id={} (treat as timeout/cancel): {}",
                    task.task_id, e
                );
                #[cfg(unix)]
                {
                    (None, std::process::ExitStatus::from_raw(1 << 8), arts)
                }
                #[cfg(not(unix))]
                {
                    let status = std::process::Command::new("cmd")
                        .args(["/C", "exit", "1"])
                        .status()
                        .unwrap_or_else(|_| std::process::ExitStatus::from_raw(1));
                    (None, status, arts)
                }
            }
        };

    let duration_ms = start.elapsed().as_millis();
    let exit_code: i32 = {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            match status.code() {
                Some(c) => c,
                None => status.signal().map(|s| 128 + s).unwrap_or(1),
            }
        }
        #[cfg(not(unix))]
        {
            status.code().unwrap_or(1)
        }
    };

    // Stop heartbeat and attempt to release lock (best-effort)
    stop_flag.store(true, Ordering::Relaxed);
    let _ = crate::executor::lock::release_lock(task.task_id, &task.task_lock);
    eprintln!(
        "[executor] finished task_id={} in {} ms (releasing lock)",
        task.task_id, duration_ms
    );

    // Upload logs to GCS (only if artifacts exist)
    eprintln!(
        "[executor] uploading logs for task_id={} to gs://icfpc2025-data/logs/{}/",
        task.task_id, task.task_id
    );
    upload_logs(task.task_id, &artifacts)?;
    eprintln!("[executor] uploaded logs for task_id={}", task.task_id);

    if let Some(s) = score {
        eprintln!(
            "[executor] detected score for task_id={}: {}",
            task.task_id, s
        );
    } else {
        eprintln!(
            "[executor] no <UNAGI> score found for task_id={}",
            task.task_id
        );
    }

    Ok((score, exit_code, duration_ms))
}

/// Updates the task with the given score and duration, and releases the lock.
pub fn update_task(
    task: &Task,
    score: Option<i64>,
    exit_code: i32,
    duration_ms: u128,
) -> Result<()> {
    eprintln!(
        "[executor] updating task_id={} score={:?} exit_code={} duration_ms={}",
        task.task_id, score, exit_code, duration_ms
    );
    let _ = sql::exec(
        r#"
        UPDATE tasks
        SET task_score = :task_score,
            task_exit_code = :task_exit_code,
            task_duration_ms = :task_duration_ms,
            task_locked = NULL
        WHERE task_id = :task_id AND task_lock = :task_lock
        "#,
        params! {
            "task_score" => score,
            "task_exit_code" => exit_code,
            "task_duration_ms" => (duration_ms as i64),
            "task_id" => task.task_id,
            "task_lock" => &task.task_lock,
        },
    )?;
    eprintln!("[executor] updated task_id={} (lock cleared)", task.task_id);
    Ok(())
}

fn gen_lock_token() -> String {
    let buf: [u8; 16] = rand::random();
    hex::encode(buf)
}

fn upload_logs(task_id: i64, artifacts: &run::Artifacts) -> Result<()> {
    // Build object names
    let bucket = "icfpc2025-data";
    let prefix = format!("logs/{}/", task_id);
    let stdout_name = format!("{}stdout.jsonl", prefix);
    let stderr_name = format!("{}stderr.jsonl", prefix);

    // Read files
    let stdout_bytes = std::fs::read(artifacts.stdout_file())?;
    let stderr_bytes = std::fs::read(artifacts.stderr_file())?;

    // Use a local runtime to perform async uploads
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        if stdout_bytes.is_empty() {
            eprintln!(
                "[executor] skipping upload (stdout is empty) for task_id={}",
                task_id
            );
        } else {
            let _ = crate::gcp::gcs::upload_object(
                bucket,
                &stdout_name,
                &stdout_bytes,
                "application/x-ndjson",
            )
            .await?;
        }

        if stderr_bytes.is_empty() {
            eprintln!(
                "[executor] skipping upload (stderr is empty) for task_id={}",
                task_id
            );
        } else {
            let _ = crate::gcp::gcs::upload_object(
                bucket,
                &stderr_name,
                &stderr_bytes,
                "application/x-ndjson",
            )
            .await?;
        }
        anyhow::Ok(())
    })
}

fn current_hostname() -> String {
    if let Ok(os) = hostname::get()
        && let Ok(s) = os.into_string()
        && !s.is_empty()
    {
        return s;
    }
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string())
}

fn prepare_agent_bin(agent_url: &str, root_dir: &Path) -> anyhow::Result<()> {
    use crate::gcp::gcs::{download_object, get_object_metadata, parse_gs_url};
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use std::fs;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let (bucket, object) = parse_gs_url(agent_url)?;
    let rt = tokio::runtime::Runtime::new()?;
    let meta = rt.block_on(get_object_metadata(&bucket, &object))?;
    let md5_b64 = meta
        .md5_hash
        .ok_or_else(|| anyhow::anyhow!("md5Hash missing for {}", agent_url))?;
    let md5_bytes = BASE64
        .decode(md5_b64.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid md5Hash base64: {}", e))?;
    let md5_hex = hex::encode(&md5_bytes);

    let cache_path = Path::new("/var/tmp").join(format!("agent-bin-{}", md5_hex));
    let mut use_cache = false;
    if cache_path.exists() {
        let bytes = fs::read(&cache_path)?;
        let sum = md5::compute(&bytes);
        if format!("{:x}", sum) == md5_hex {
            use_cache = true;
        } else {
            let _ = fs::remove_file(&cache_path);
        }
    }

    if !use_cache {
        let bytes = rt.block_on(download_object(&bucket, &object))?;
        let sum = md5::compute(&bytes);
        if format!("{:x}", sum) != md5_hex {
            anyhow::bail!("downloaded md5 mismatch for {}", agent_url);
        }
        let tmp_name = format!(
            "agent-tmp-{}-{:<08x}",
            std::process::id(),
            rand::random::<u32>()
        );
        let tmp_path = Path::new("/var/tmp").join(tmp_name);
        {
            let mut f = fs::File::create(&tmp_path)?;
            f.write_all(&bytes)?;
        }
        #[cfg(unix)]
        let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o755));
        match fs::rename(&tmp_path, &cache_path) {
            Ok(()) => {}
            Err(_) => {
                // If another process raced and created a correct cache, accept it
                if cache_path.exists()
                    && format!("{:x}", md5::compute(fs::read(&cache_path)?)) == md5_hex
                {
                    let _ = fs::remove_file(&tmp_path);
                } else {
                    return Err(anyhow::anyhow!("failed to finalize cache file"));
                }
            }
        }
    }

    // Copy to artifacts root as main and set executable
    let dest = root_dir.join("main");
    fs::copy(&cache_path, &dest)?;
    #[cfg(unix)]
    let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    Ok(())
}
