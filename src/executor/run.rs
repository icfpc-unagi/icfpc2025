use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant};

/// Run the provided bash script, capture stdout/stderr as JSONL to files, and
/// return the last <UNAGI>: JSON score and exit status. Honors timeout and a
/// cancellation flag (when set to true, terminates the child).
pub fn run_command(
    script: &str,
    stdout_path: &Path,
    stderr_path: &Path,
    timeout: Duration,
    cancel: Arc<AtomicBool>,
) -> Result<(Option<i64>, std::process::ExitStatus)> {
    // Open logs and spawn child
    let (stdout_file, stderr_file) = open_logs(stdout_path, stderr_path)?;
    let mut child = spawn_bash(script)?;

    // Take pipes and spawn reader threads
    let out_pipe = child.stdout.take().unwrap();
    let err_pipe = child.stderr.take().unwrap();
    let last_json: Arc<Mutex<Option<JsonValue>>> = Arc::new(Mutex::new(None));
    let out_thread = spawn_stdout_thread(out_pipe, stdout_file, Arc::clone(&last_json));
    let err_thread = spawn_stderr_thread(err_pipe, stderr_file);

    // Supervise
    let (terminated_due_to_timeout_or_cancel, status_opt) =
        supervise_child(&mut child, timeout, cancel)?;

    // Join readers with bounded wait
    let extra = Duration::from_secs(1);
    join_with_timeout(out_thread, extra);
    join_with_timeout(err_thread, extra);

    // Result
    let score = extract_score(&last_json);
    if terminated_due_to_timeout_or_cancel && status_opt.is_none() {
        anyhow::bail!("timeout/cancel wait exceeded 1s");
    }
    let status = status_opt.expect("status should be present unless bailed");
    Ok((score, status))
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

fn spawn_bash(script: &str) -> Result<Child> {
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn bash")?;
    Ok(child)
}

fn spawn_stdout_thread(
    out_pipe: ChildStdout,
    mut stdout_file: File,
    last_json: Arc<Mutex<Option<JsonValue>>>,
) -> std::thread::JoinHandle<Result<()>> {
    let mut out_reader = BufReader::new(out_pipe);
    std::thread::spawn(move || -> Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = out_reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            write_jsonl(&mut stdout_file, &line)?;
            if let Some(json) = parse_unagi_line(&line) {
                if let Ok(mut slot) = last_json.lock() {
                    *slot = Some(json);
                }
            }
        }
        Ok(())
    })
}

fn spawn_stderr_thread(
    err_pipe: ChildStderr,
    mut stderr_file: File,
) -> std::thread::JoinHandle<Result<()>> {
    let mut err_reader = BufReader::new(err_pipe);
    std::thread::spawn(move || -> Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = err_reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            write_jsonl(&mut stderr_file, &line)?;
        }
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

fn write_jsonl(file: &mut File, text: &str) -> Result<()> {
    let ts = chrono::Utc::now().to_rfc3339();
    let obj = serde_json::json!({
        "timestamp": ts,
        "text": text.trim_end_matches(['\n', '\r'])
    });
    let line = serde_json::to_string(&obj)?;
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn run_command_captures_and_parses_unagi() -> Result<()> {
        // Prepare temp paths
        let base = std::env::temp_dir().join(format!("executor_test_{}", uuid()));
        fs::create_dir_all(&base)?;
        let stdout_path = base.join("stdout.jsonl");
        let stderr_path = base.join("stderr.jsonl");

        // A script that prints to stdout/stderr and an UNAGI line
        let script = "echo out1; echo err1 1>&2; echo '<UNAGI>: {\"score\": 123}';";
        let (score, status) = run_command(
            script,
            &stdout_path,
            &stderr_path,
            Duration::from_secs(5),
            Arc::new(AtomicBool::new(false)),
        )?;

        assert!(status.success());
        assert_eq!(score, Some(123));
        let out = fs::read_to_string(&stdout_path)?;
        let err = fs::read_to_string(&stderr_path)?;
        assert!(out.contains("out1"));
        assert!(out.contains("<UNAGI>"));
        assert!(err.contains("err1"));
        Ok(())
    }

    #[test]
    #[ignore]
    fn run_command_times_out_and_kills() -> Result<()> {
        // Prepare temp paths
        let base = std::env::temp_dir().join(format!("executor_timeout_{}", uuid()));
        fs::create_dir_all(&base)?;
        let stdout_path = base.join("stdout.jsonl");
        let stderr_path = base.join("stderr.jsonl");

        // Script that sleeps longer than timeout
        let script = "echo start; sleep 3; echo done";
        let (score, status) = run_command(
            script,
            &stdout_path,
            &stderr_path,
            Duration::from_millis(800),
            Arc::new(AtomicBool::new(false)),
        )?;

        // Should have timed out and been killed; not success
        assert!(!status.success());
        // No UNAGI score expected
        assert_eq!(score, None);
        // Ensure at least the initial output got captured before timeout
        let out = fs::read_to_string(&stdout_path)?;
        assert!(out.contains("start"));
        Ok(())
    }

    fn uuid() -> String {
        let buf: [u8; 8] = rand::random();
        hex::encode(buf)
    }
}
