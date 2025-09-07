use anyhow::{Context, Result};
use chrono::{Datelike, Timelike};
use serde_json::Value as JsonValue;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

/// Run the provided bash script, capture stdout/stderr as JSONL to files under
/// a temporary directory, and return the last <UNAGI>: JSON score, exit status,
/// and created artifacts. Honors timeout and a cancellation flag.
pub fn run_command(
    script: &str,
    timeout: Duration,
    cancel: Arc<AtomicBool>,
) -> Result<(Option<i64>, std::process::ExitStatus, Artifacts)> {
    // Prepare artifacts and directories
    let artifacts = create_artifacts_dir()?;
    fs::create_dir_all(artifacts.root_dir())?;
    fs::create_dir_all(artifacts.log_dir())?;

    // Open logs and spawn child using root as cwd
    let (stdout_file, stderr_file) = open_logs(&artifacts.stdout_file(), &artifacts.stderr_file())?;
    let mut child = spawn_bash(script, artifacts.root_dir())?;

    // Take pipes and spawn reader threads
    let out_pipe = child.stdout.take().context("child missing stdout pipe")?;
    let err_pipe = child.stderr.take().context("child missing stderr pipe")?;
    let last_json: Arc<Mutex<Option<JsonValue>>> = Arc::new(Mutex::new(None));
    let out_thread = spawn_log_thread(out_pipe, stdout_file, Some(Arc::clone(&last_json)));
    let err_thread = spawn_log_thread(err_pipe, stderr_file, None);

    // Supervise
    let (terminated_due_to_timeout_or_cancel, status_opt) =
        supervise_child(&mut child, timeout, cancel)?;

    // Join readers with bounded wait
    let extra = Duration::from_secs(7);
    join_with_timeout(out_thread, extra);
    join_with_timeout(err_thread, extra);

    // Result
    let score = extract_score(&last_json);
    if terminated_due_to_timeout_or_cancel && status_opt.is_none() {
        anyhow::bail!("timeout/cancel wait exceeded 1s");
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
) -> std::thread::JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        let mut reader = BufReader::new(pipe);
        let mut writer = BufWriter::new(file);
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let mut bytes_written: usize = 0;
        let max_bytes: usize = 100 * 1024 * 1024; // 100MB
        let mut last_flush = Instant::now();
        loop {
            buf.clear();
            let n = reader.read_until(b'\n', &mut buf)?;
            if n == 0 {
                break;
            }
            let line = String::from_utf8_lossy(&buf);
            if bytes_written < max_bytes {
                let wrote = write_jsonl(&mut writer, &line)?;
                bytes_written = bytes_written.saturating_add(wrote);
            }
            if let Some(ref slot_arc) = last_json
                && let Some(json) = parse_unagi_line(&line)
                && let Ok(mut slot) = slot_arc.lock()
            {
                *slot = Some(json);
            }
            if last_flush.elapsed() >= Duration::from_millis(500) {
                let _ = writer.flush();
                last_flush = Instant::now();
            }
        }
        let _ = writer.flush();
        Ok(())
    })
}

fn supervise_child(
    child: &mut Child,
    timeout: Duration,
    cancel: Arc<AtomicBool>,
) -> Result<(bool, Option<ExitStatus>)> {
    let start = Instant::now();
    let mut terminated_due_to_timeout_or_cancel = false;
    let status_opt = loop {
        if cancel.load(Ordering::Relaxed) || start.elapsed() > timeout {
            terminated_due_to_timeout_or_cancel = true;
            kill_child_group(child);
            // bounded wait: up to +1s
            let deadline = Instant::now() + Duration::from_secs(1);
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

fn write_jsonl(file: &mut dyn Write, text: &str) -> Result<usize> {
    let ts = chrono::Utc::now().to_rfc3339();
    let obj = serde_json::json!({
        "timestamp": ts,
        "text": text.trim_end_matches(['\n', '\r'])
    });
    let line = serde_json::to_string(&obj)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(line.len() + 1)
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
    let base = std::env::temp_dir().join(format!("executor_run_{}", ts));
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
            Duration::from_secs(5),
            Arc::new(AtomicBool::new(false)),
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
    #[ignore]
    fn run_command_times_out_and_kills() -> Result<()> {
        // Script that sleeps longer than timeout
        let script = "echo start; sleep 3; echo done";
        let (score, status, artifacts) = run_command(
            script,
            Duration::from_millis(800),
            Arc::new(AtomicBool::new(false)),
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
