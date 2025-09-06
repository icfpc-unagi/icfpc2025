use anyhow::{Context, Result};
use mysql::params;
use std::fs::create_dir_all;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::sql;

pub mod lock;
pub mod run;

/// Information required to execute a task.
pub struct Task {
    pub task_id: i64,
    pub problem_name: String,
    pub problem_variant: i64,
    pub agent_name: String,
    pub agent_code: String,
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
            t.task_locked = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 SECOND)
        "#,
        params! { "task_lock" => &lock_token },
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
        SELECT t.task_id, t.problem_name, t.problem_variant, a.agent_name, a.agent_code
        FROM tasks t
        JOIN agents a ON a.agent_id = :agent_id
        WHERE t.task_id = :task_id
        "#,
        params! { "agent_id" => agent_id, "task_id" => task_id },
    )?
    .context("Agent for task not found")?;

    let agent_name: String = row.get("agent_name")?;
    let agent_code: String = row.get("agent_code")?;

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
pub fn run_task(task: &Task) -> Result<(Option<i64>, u128)> {
    // Prepare command by substituting placeholders
    let mut script = task.agent_code.clone();
    script = script.replace("\r", "");
    script = script.replace("{{problem_name}}", &task.problem_name);
    script = script.replace("{{problem_variant}}", &task.problem_variant.to_string());
    script = script.replace("{{task_id}}", &task.task_id.to_string());
    script = script.replace("{{agent_name}}", &task.agent_name);

    // Output directory and files
    let base_dir: PathBuf = ["target", "logs", &task.task_id.to_string()]
        .iter()
        .collect();
    create_dir_all(&base_dir)?;
    let stdout_path = base_dir.join("stdout.jsonl");
    let stderr_path = base_dir.join("stderr.jsonl");

    eprintln!(
        "[executor] starting task_id={} (logs: {:?}, {:?})",
        task.task_id, stdout_path, stderr_path
    );
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
    let (score, _status) = match run::run_command(
        &script,
        &stdout_path,
        &stderr_path,
        Duration::from_secs(600),
        Arc::clone(&cancel),
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "[executor] run_command failed for task_id={} (treat as timeout/cancel): {}",
                task.task_id, e
            );
            #[cfg(unix)]
            {
                (None, std::process::ExitStatus::from_raw(1 << 8))
            }
            #[cfg(not(unix))]
            {
                let status = std::process::Command::new("cmd")
                    .args(["/C", "exit", "1"])
                    .status()
                    .unwrap_or_else(|_| std::process::ExitStatus::from_raw(1));
                (None, status)
            }
        }
    };

    let duration_ms = start.elapsed().as_millis();

    // Stop heartbeat and attempt to release lock (best-effort)
    stop_flag.store(true, Ordering::Relaxed);
    let _ = crate::executor::lock::release_lock(task.task_id, &task.task_lock);
    eprintln!(
        "[executor] finished task_id={} in {} ms (releasing lock)",
        task.task_id, duration_ms
    );

    // Upload logs to GCS
    eprintln!(
        "[executor] uploading logs for task_id={} to gs://icfpc2025-data/logs/{}/",
        task.task_id, task.task_id
    );
    upload_logs(task.task_id, &stdout_path, &stderr_path)?;
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

    Ok((score, duration_ms))
}

/// Updates the task with the given score and duration, and releases the lock.
pub fn update_task(task: &Task, score: Option<i64>, duration_ms: u128) -> Result<()> {
    eprintln!(
        "[executor] updating task_id={} score={:?} duration_ms={}",
        task.task_id, score, duration_ms
    );
    let _ = sql::exec(
        r#"
        UPDATE tasks
        SET task_score = :task_score,
            task_duration_ms = :task_duration_ms,
            task_locked = NULL
        WHERE task_id = :task_id AND task_lock = :task_lock
        "#,
        params! {
            "task_score" => score,
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

fn upload_logs(task_id: i64, stdout_path: &PathBuf, stderr_path: &PathBuf) -> Result<()> {
    // Build object names
    let bucket = "icfpc2025-data";
    let prefix = format!("logs/{}/", task_id);
    let stdout_name = format!("{}stdout.jsonl", prefix);
    let stderr_name = format!("{}stderr.jsonl", prefix);

    // Read files
    let stdout_bytes = std::fs::read(stdout_path)?;
    let stderr_bytes = std::fs::read(stderr_path)?;

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
