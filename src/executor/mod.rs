use anyhow::{Context, Result};
use mysql::params;
use serde_json::Value as JsonValue;
use std::fs::{File, create_dir_all};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::sql;

pub mod lock;

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

    let mut stdout_file = File::create(&stdout_path)?;
    let mut stderr_file = File::create(&stderr_path)?;

    eprintln!(
        "[executor] starting task_id={} (logs: {:?}, {:?})",
        task.task_id, stdout_path, stderr_path
    );

    // Spawn `bash -lc` to run the script
    let mut child = Command::new("bash")
        .arg("-lc")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn bash")?;

    let start = Instant::now();
    let mut last_unagi_json: Option<JsonValue> = None;

    // Readers for stdout and stderr
    let mut out_reader = BufReader::new(child.stdout.take().unwrap());
    let mut err_reader = BufReader::new(child.stderr.take().unwrap());

    // Set up heartbeat that can terminate the process if lock extension fails
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    let stop_flag = Arc::new(AtomicBool::new(false));
    let child_arc = Arc::new(Mutex::new(child));
    let hb_task_id = task.task_id;
    let hb_lock = task.task_lock.clone();
    let hb_stop = Arc::clone(&stop_flag);
    let hb_child = Arc::clone(&child_arc);
    let _hb = std::thread::spawn(move || {
        while !hb_stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_secs(10));
            match crate::executor::lock::extend_lock(hb_task_id, &hb_lock) {
                Ok(true) => {}
                Ok(false) | Err(_) => {
                    eprintln!(
                        "[executor] lock extend failed for task_id={}, terminating process",
                        hb_task_id
                    );
                    if let Ok(mut ch) = hb_child.lock() {
                        let _ = ch.kill();
                    }
                    break;
                }
            }
        }
    });

    // Simple loop to read both streams without blocking indefinitely on one.
    // We alternate attempts using non-blocking `fill_buf` checks.
    let timeout = Duration::from_secs(600);
    loop {
        // Check for timeout
        if start.elapsed() > timeout {
            eprintln!(
                "[executor] timeout reached (600s) for task_id={}, killing process",
                task.task_id
            );
            if let Ok(mut ch) = child_arc.lock() {
                let _ = ch.kill();
            }
            break;
        }

        let mut progressed = false;

        // Read one line from stdout if available
        let mut line = String::new();
        let n = out_reader.read_line(&mut line)?;
        if n > 0 {
            progressed = true;
            write_jsonl(&mut stdout_file, &line)?;
            if let Some(json) = parse_unagi_line(&line) {
                last_unagi_json = Some(json);
            }
        }

        // Read one line from stderr if available
        let mut eline = String::new();
        let n = err_reader.read_line(&mut eline)?;
        if n > 0 {
            progressed = true;
            write_jsonl(&mut stderr_file, &eline)?;
        }

        // If process exited and both streams are drained, break
        let status_opt = {
            if let Ok(mut ch) = child_arc.lock() {
                ch.try_wait()?
            } else {
                None
            }
        };
        match status_opt {
            Some(status) => {
                eprintln!(
                    "[executor] process exited for task_id={} status={}",
                    task.task_id, status
                );
                // Drain remaining lines
                let mut tmp = String::new();
                while out_reader.read_line(&mut tmp)? > 0 {
                    write_jsonl(&mut stdout_file, &tmp)?;
                    if let Some(json) = parse_unagi_line(&tmp) {
                        last_unagi_json = Some(json);
                    }
                    tmp.clear();
                }
                while err_reader.read_line(&mut tmp)? > 0 {
                    write_jsonl(&mut stderr_file, &tmp)?;
                    tmp.clear();
                }
                break;
            }
            None => {
                if !progressed {
                    // Avoid busy loop
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

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

    // Extract score from last UNAGI JSON
    let score = last_unagi_json
        .and_then(|v| v.get("score").cloned())
        .and_then(|v| v.as_i64());
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

fn write_jsonl(file: &mut File, text: &str) -> Result<()> {
    let ts = chrono::Utc::now().to_rfc3339();
    let obj = serde_json::json!({
        "timestamp": ts,
        "text": text.trim_end_matches(['\n', '\r'])
    });
    let line = serde_json::to_string(&obj)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?; // try to avoid buffering
    Ok(())
}

fn parse_unagi_line(line: &str) -> Option<JsonValue> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("<UNAGI>:") {
        serde_json::from_str::<JsonValue>(rest.trim()).ok()
    } else {
        None
    }
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
