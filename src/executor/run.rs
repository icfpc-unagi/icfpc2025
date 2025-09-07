use anyhow::{Context, Result};
use chrono::{Datelike, Timelike};
use serde_json::Value as JsonValue;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

/// Run the provided bash script, capture stdout/stderr as JSONL to files under
/// a temporary directory, and return the last <UNAGI>: JSON score, exit status,
/// and created artifacts. Honors timeout and a cancellation flag.
#[derive(Clone)]
pub struct RunOptions {
    pub log_max_bytes: usize,
    pub log_tail_bytes: usize,
    pub flush_interval: Duration,
    pub join_grace: Duration,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            log_max_bytes: 100 * 1024 * 1024, // 100MB
            log_tail_bytes: 10 * 1024 * 1024, // 10MB
            flush_interval: Duration::from_millis(500),
            join_grace: Duration::from_secs(7),
        }
    }
}

pub fn run_command<F>(
    script: &str,
    cancel: Arc<AtomicBool>,
    prepare: F,
    opts: &RunOptions,
) -> Result<(Option<i64>, std::process::ExitStatus, Artifacts)>
where
    F: FnOnce(&Artifacts) -> Result<()>,
{
    // Prepare artifacts and directories
    let artifacts = create_artifacts_dir()?;
    fs::create_dir_all(artifacts.root_dir())?;
    fs::create_dir_all(artifacts.log_dir())?;

    // Allow caller to prepare files under the temp directory before execution
    prepare(&artifacts)?;

    // Open logs and spawn child using root as cwd
    let (stdout_file, stderr_file) = open_logs(&artifacts.stdout_file(), &artifacts.stderr_file())?;
    let mut child = spawn_bash(script, artifacts.root_dir())?;

    // Take pipes and spawn reader threads
    let out_pipe = child.stdout.take().context("child missing stdout pipe")?;
    let err_pipe = child.stderr.take().context("child missing stderr pipe")?;
    let last_json: Arc<Mutex<Option<JsonValue>>> = Arc::new(Mutex::new(None));
    let out_thread = spawn_log_thread(
        out_pipe,
        stdout_file,
        Some(Arc::clone(&last_json)),
        opts.clone(),
    );
    let err_thread = spawn_log_thread(err_pipe, stderr_file, None, opts.clone());

    // Supervise
    let (terminated_due_to_timeout_or_cancel, status_opt) =
        supervise_child(&mut child, None, cancel)?;

    // Join readers with bounded wait
    let extra = opts.join_grace;
    join_with_timeout(out_thread, extra);
    join_with_timeout(err_thread, extra);

    // Result
    let mut score = extract_score(&last_json);
    if score.is_none() {
        score = extract_score_from_log(&artifacts.stdout_file())
            .ok()
            .flatten();
    }
    if terminated_due_to_timeout_or_cancel && status_opt.is_none() {
        // Could not obtain child status within the bounded wait; synthesize a failure status.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            let status = std::process::ExitStatus::from_raw(1 << 8);
            return Ok((score, status, artifacts));
        }
        #[cfg(not(unix))]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "exit", "1"])
                .status()
                .unwrap_or_else(|_| unsafe { std::mem::zeroed() });
            return Ok((score, status, artifacts));
        }
    }
    let status = status_opt.expect("status should be present unless bailed");
    Ok((score, status, artifacts))
}

#[cfg(unix)]
fn kill_child_group(child: &mut std::process::Child) {
    // Kill the whole process group (-pid)
    unsafe {
        let pid = child.id() as i32;
        libc::kill(-pid, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_child_group(child: &mut std::process::Child) {
    let _ = child.kill();
}

fn open_logs(stdout_path: &Path, stderr_path: &Path) -> Result<(File, File)> {
    let stdout_file = File::create(stdout_path)?;
    let stderr_file = File::create(stderr_path)?;
    Ok((stdout_file, stderr_file))
}

fn spawn_bash(script: &str, workdir: &Path) -> Result<Child> {
    let mut cmd = Command::new("bash");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    let child = cmd
        .arg("-lc")
        .arg(script)
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn bash")?;
    Ok(child)
}

fn spawn_log_thread<R: std::io::Read + Send + 'static>(
    pipe: R,
    file: File,
    last_json: Option<Arc<Mutex<Option<JsonValue>>>>,
    opts: RunOptions,
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        let mut reader = BufReader::new(pipe);
        let writer = Arc::new(Mutex::new(BufWriter::new(file)));
        let stop = Arc::new(AtomicBool::new(false));
        let writer_for_flush = Arc::clone(&writer);
        let stop_for_flush = Arc::clone(&stop);
        let flusher = std::thread::spawn(move || {
            while !stop_for_flush.load(Ordering::Relaxed) {
                std::thread::sleep(opts.flush_interval);
                if let Ok(mut w) = writer_for_flush.lock() {
                    let _ = w.flush();
                }
            }
            if let Ok(mut w) = writer_for_flush.lock() {
                let _ = w.flush();
            }
        });
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let mut bytes_written: usize = 0;
        let max_bytes: usize = opts.log_max_bytes;
        let tail_cap: usize = opts.log_tail_bytes;
        let mut tail: VecDeque<u8> = VecDeque::with_capacity(tail_cap);
        let mut overflow_total: usize = 0;
        loop {
            buf.clear();
            let n = reader.read_until(b'\n', &mut buf)?;
            if n == 0 {
                break;
            }
            let line = String::from_utf8_lossy(&buf);
            let rec = encode_jsonl(&line)?; // encoded JSONL bytes for this line
            if bytes_written < max_bytes {
                let mut w = writer.lock().unwrap();
                w.write_all(&rec)?;
                bytes_written = bytes_written.saturating_add(rec.len());
            } else {
                overflow_total = overflow_total.saturating_add(rec.len());
                // keep only the last tail_cap bytes in tail
                if rec.len() >= tail_cap {
                    tail.clear();
                    tail.extend(rec[rec.len() - tail_cap..].iter().copied());
                } else {
                    // if exceeding capacity, pop from front
                    let needed = rec.len();
                    let free = tail_cap.saturating_sub(tail.len());
                    if needed > free {
                        let to_drop = needed - free;
                        for _ in 0..to_drop {
                            let _ = tail.pop_front();
                        }
                    }
                    tail.extend(rec);
                }
            }
            if let Some(ref slot_arc) = last_json
                && let Some(json) = parse_unagi_line(&line)
                && let Ok(mut slot) = slot_arc.lock()
            {
                *slot = Some(json);
            }
        }
        // If we had overflow, write a truncation marker and the tail
        if overflow_total > 0 {
            let truncated_bytes = overflow_total.saturating_sub(tail.len());
            let marker_rec = encode_truncated(truncated_bytes)?;
            let mut w = writer.lock().unwrap();
            w.write_all(&marker_rec)?;
            if !tail.is_empty() {
                // Collect tail into contiguous buffer
                let mut tail_buf = Vec::with_capacity(tail.len());
                tail_buf.extend(tail);
                w.write_all(&tail_buf)?;
            }
        }
        stop.store(true, Ordering::Relaxed);
        let _ = flusher.join();
        Ok(())
    })
}

fn supervise_child(
    child: &mut Child,
    timeout: Option<Duration>,
    cancel: Arc<AtomicBool>,
) -> Result<(bool, Option<ExitStatus>)> {
    let start = Instant::now();
    let mut terminated_due_to_timeout_or_cancel = false;
    let status_opt = loop {
        let timed_out = timeout.map(|t| start.elapsed() > t).unwrap_or(false);
        if cancel.load(Ordering::Relaxed) || timed_out {
            terminated_due_to_timeout_or_cancel = true;
            kill_child_group(child);
            // bounded wait: allow up to +5s for process to terminate
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut waited = None;
            while Instant::now() < deadline {
                if let Some(st) = child.try_wait()? {
                    waited = Some(st);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            break waited;
        }
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    Ok((terminated_due_to_timeout_or_cancel, status_opt))
}

/// Convenience wrapper to add a timeout to run_command.
pub fn run_command_with_timeout<F>(
    script: &str,
    timeout: Duration,
    cancel: Arc<AtomicBool>,
    prepare: F,
    opts: &RunOptions,
) -> Result<(Option<i64>, std::process::ExitStatus, Artifacts)>
where
    F: FnOnce(&Artifacts) -> Result<()>,
{
    // Use cancel flag to implement timeout
    let cancel_for_timer = Arc::clone(&cancel);
    let (tx, rx) = mpsc::channel();
    let _timer = std::thread::spawn(move || {
        if rx.recv_timeout(timeout).is_err() {
            cancel_for_timer.store(true, Ordering::Relaxed);
        }
    });

    let result = run_command(script, cancel, prepare, opts);
    let _ = tx.send(()); // stop timer if still waiting
    // Do not join the timer thread here: in case of a bug that blocks the thread,
    // joining could hang the caller. Dropping the JoinHandle lets the thread exit
    // on its own shortly after receiving the stop signal (or the timeout).
    result
}

fn join_with_timeout(h: std::thread::JoinHandle<Result<()>>, dur: Duration) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = h.join();
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(dur);
}

fn extract_score(last_json: &Arc<Mutex<Option<JsonValue>>>) -> Option<i64> {
    last_json
        .lock()
        .ok()
        .and_then(|v| v.clone())
        .and_then(|v| v.get("score").cloned())
        .and_then(|v| v.as_i64())
}

fn extract_score_from_log(path: &Path) -> Result<Option<i64>> {
    let file = File::open(path).context("open stdout log for score parse")?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut last: Option<i64> = None;
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                if let Some(rest) = text.trim_start().strip_prefix("<UNAGI>:") {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(rest.trim()) {
                        if let Some(sc) = obj.get("score").and_then(|s| s.as_i64()) {
                            last = Some(sc);
                        }
                    }
                }
            }
        }
    }
    Ok(last)
}

fn encode_jsonl(text: &str) -> Result<Vec<u8>> {
    let ts = chrono::Utc::now().to_rfc3339();
    let obj = serde_json::json!({
        "timestamp": ts,
        "text": text.trim_end_matches(['\n', '\r'])
    });
    let line = serde_json::to_vec(&obj)?;
    let mut out = Vec::with_capacity(line.len() + 1);
    out.extend_from_slice(&line);
    out.push(b'\n');
    Ok(out)
}

fn encode_truncated(truncated_bytes: usize) -> Result<Vec<u8>> {
    let ts = chrono::Utc::now().to_rfc3339();
    let obj = serde_json::json!({
        "timestamp": ts,
        "truncated": truncated_bytes,
    });
    let line = serde_json::to_vec(&obj)?;
    let mut out = Vec::with_capacity(line.len() + 1);
    out.extend_from_slice(&line);
    out.push(b'\n');
    Ok(out)
}

fn parse_unagi_line(line: &str) -> Option<JsonValue> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("<UNAGI>:") {
        serde_json::from_str::<JsonValue>(rest.trim()).ok()
    } else {
        None
    }
}

fn create_artifacts_dir() -> Result<Artifacts> {
    let now = chrono::Utc::now();
    let ts = format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}_{:06}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        now.timestamp_subsec_micros()
    );
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let base = std::env::temp_dir().join(format!("executor_run_{}_{}_{}", ts, pid, c));
    let root = base.join("root");
    let log = base.join("log");
    fs::create_dir_all(&base)?;
    Ok(Artifacts {
        base_dir: base,
        root_dir: root,
        log_dir: log,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn run_command_captures_and_parses_unagi() -> Result<()> {
        // A script that prints to stdout/stderr and an UNAGI line
        let script = "echo out1; echo err1 1>&2; echo '<UNAGI>: {\"score\": 123}';";
        let (score, status, artifacts) = run_command(
            script,
            Arc::new(AtomicBool::new(false)),
            |_| Ok(()),
            &RunOptions::default(),
        )?;

        assert!(status.success());
        assert_eq!(score, Some(123));
        let out = fs::read_to_string(artifacts.stdout_file())?;
        let err = fs::read_to_string(artifacts.stderr_file())?;
        assert!(out.contains("out1"));
        assert!(out.contains("<UNAGI>"));
        assert!(err.contains("err1"));
        Ok(())
    }

    #[test]
    fn run_command_times_out_and_kills() -> Result<()> {
        // Script that sleeps longer than timeout
        let script = "echo start; sleep 3; echo done";
        let (score, status, artifacts) = run_command_with_timeout(
            script,
            Duration::from_millis(800),
            Arc::new(AtomicBool::new(false)),
            |_| Ok(()),
            &RunOptions::default(),
        )?;

        // Should have timed out and been killed; not success
        assert!(!status.success());
        // No UNAGI score expected
        assert_eq!(score, None);
        // Ensure at least the initial output got captured before timeout
        let out = fs::read_to_string(artifacts.stdout_file())?;
        assert!(out.contains("start"));
        Ok(())
    }

    #[test]
    fn run_command_prepares_files_via_callback() -> Result<()> {
        // Prepare callback to create a file under root, then cat it
        let script = "cat prepared.txt; echo '<UNAGI>: {\"score\": 0}'";
        let (score, status, artifacts) = run_command(
            script,
            Arc::new(AtomicBool::new(false)),
            |arts| {
                let path = arts.root_dir().join("prepared.txt");
                std::fs::write(&path, b"prepared-content\n")?;
                Ok(())
            },
            &RunOptions::default(),
        )?;

        assert!(status.success());
        assert_eq!(score, Some(0));
        let out = fs::read_to_string(artifacts.stdout_file())?;
        assert!(out.contains("prepared-content"));
        Ok(())
    }

    #[test]
    fn run_command_nonexistent_command() -> Result<()> {
        let script = "definitely_nonexistent_command_xyz_123";
        let (score, status, artifacts) = run_command(
            script,
            Arc::new(AtomicBool::new(false)),
            |_| Ok(()),
            &RunOptions::default(),
        )?;
        assert!(!status.success());
        assert_eq!(score, None);
        let err = fs::read_to_string(artifacts.stderr_file())?;
        assert!(!err.is_empty(), "stderr should capture the error message");
        Ok(())
    }

    #[test]
    fn run_command_cancel_terminates() -> Result<()> {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_set = Arc::clone(&cancel);
        // Flip cancel shortly after start
        let _t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            cancel_set.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let script = "sleep 5; echo should_not_happen";
        let (score, status, _artifacts) = run_command(
            script,
            Arc::clone(&cancel),
            |_| Ok(()),
            &RunOptions::default(),
        )?;
        assert!(!status.success());
        assert_eq!(score, None);
        Ok(())
    }

    #[test]
    fn run_command_uses_last_unagi_score() -> Result<()> {
        let script = "echo '<UNAGI>: {\\\"score\\\": 1}'; \
                      echo '<UNAGI>: {\\\"score\\\": 2}'; \
                      echo '<UNAGI>: {\\\"score\\\": 3}'";
        let (score, status, artifacts) = run_command(
            script,
            Arc::new(AtomicBool::new(false)),
            |_| Ok(()),
            &RunOptions::default(),
        )?;
        assert!(status.success());
        let score2 = score.or_else(|| {
            extract_score_from_log(&artifacts.stdout_file())
                .ok()
                .flatten()
        });
        assert_eq!(score2, Some(3));
        Ok(())
    }

    #[test]
    #[ignore]
    fn artifacts_cleanup_on_drop() -> Result<()> {
        let script = "echo hello; echo '<UNAGI>: {\\\"score\\\": 0}'";
        let (score, status, artifacts) = run_command(
            script,
            Arc::new(AtomicBool::new(false)),
            |_| Ok(()),
            &RunOptions::default(),
        )?;
        assert!(status.success());
        let score2 = score.or_else(|| {
            extract_score_from_log(&artifacts.stdout_file())
                .ok()
                .flatten()
        });
        assert_eq!(score2, Some(0));
        let base = artifacts.base_dir().to_path_buf();
        assert!(base.exists(), "artifacts base dir should exist before drop");
        drop(artifacts);
        // Give the OS a moment if needed
        std::thread::sleep(Duration::from_millis(50));
        assert!(
            !base.exists(),
            "artifacts base dir should be removed on drop"
        );
        Ok(())
    }
    #[test]
    fn run_command_respects_options_truncates_and_tail() -> Result<()> {
        // Emit many lines plus a final marker; use small caps to force truncation
        let script = "for i in $(seq 1 5000); do echo line_$i; done; \
            echo FINAL_ONE; \
            echo FINAL_TWO; \
            echo '<UNAGI>: {\"score\": 0}'";
        let mut opts = RunOptions::default();
        opts.log_max_bytes = 2048; // ~2KB cap
        opts.log_tail_bytes = 2048; // keep enough to include FINAL_* lines
        opts.flush_interval = Duration::from_millis(50);
        opts.join_grace = Duration::from_secs(2);
        let (score, status, artifacts) =
            run_command(script, Arc::new(AtomicBool::new(false)), |_| Ok(()), &opts)?;

        assert!(status.success());
        assert_eq!(score, Some(0));
        let out = std::fs::read_to_string(artifacts.stdout_file())?;
        // Parse JSONL lines and look for truncated record and tail content
        let mut saw_truncated = false;
        let mut saw_final_one = false;
        let mut saw_final_two = false;
        for line in out.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap_or(serde_json::json!({}));
            if v.get("truncated").is_some() {
                saw_truncated = true;
            }
            if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                if text == "FINAL_ONE" {
                    saw_final_one = true;
                }
                if text == "FINAL_TWO" {
                    saw_final_two = true;
                }
            }
        }
        assert!(saw_truncated, "expected a truncated JSONL record");
        assert!(saw_final_one && saw_final_two, "expected FINAL_* in tail");
        Ok(())
    }
}
/// Artifacts created for a single run. Holds the temporary directory and
/// subdirectories for `root` (working directory) and `log` (stdout/stderr).
/// When dropped, the entire temporary directory is removed.
pub struct Artifacts {
    base_dir: PathBuf,
    root_dir: PathBuf,
    log_dir: PathBuf,
}

impl Artifacts {
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }
    pub fn stdout_file(&self) -> PathBuf {
        self.log_dir.join("stdout.jsonl")
    }
    pub fn stderr_file(&self) -> PathBuf {
        self.log_dir.join("stderr.jsonl")
    }
}

impl Drop for Artifacts {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base_dir);
    }
}
