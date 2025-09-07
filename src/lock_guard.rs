// Lock manager and shutdown guard utilities
// Manages acquiring/unlocking the Unagi API lock and ensuring best-effort unlock
// on Ctrl+C, panic, and normal program exit.

#![allow(dead_code)]

use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(feature = "reqwest")]
const LOCK_TTL: Duration = Duration::from_secs(10);
#[cfg(feature = "reqwest")]
const LOCK_RENEW_INTERVAL: Duration = Duration::from_secs(2);

#[cfg(feature = "reqwest")]
struct LockRunner {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    token: String,
}

#[cfg(feature = "reqwest")]
static LOCK_MANAGER: Lazy<Mutex<Option<LockRunner>>> = Lazy::new(|| Mutex::new(None));
#[cfg(feature = "reqwest")]
static CTRL_C_INSTALLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Panic and normal-exit unlock handling
#[cfg(feature = "reqwest")]
static PANIC_HOOK_INSTALLED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[cfg(feature = "reqwest")]
thread_local! {
    static NORMAL_EXIT_GUARD: NormalExitGuard = const { NormalExitGuard };
}

#[cfg(feature = "reqwest")]
struct NormalExitGuard;

#[cfg(feature = "reqwest")]
impl Drop for NormalExitGuard {
    fn drop(&mut self) {
        // Best-effort stop and unlock on normal thread exit (main thread on normal process exit).
        stop_lock_manager_blocking();
    }
}

#[cfg(feature = "reqwest")]
fn install_panic_hook_once() {
    if !PANIC_HOOK_INSTALLED.swap(true, Ordering::SeqCst) {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // preserve default printing
            prev(info);
            // attempt unlock on panic
            stop_lock_manager_blocking();
        }));
    }
}

/// Starts the lock manager, acquiring and renewing the lock.
#[cfg(feature = "reqwest")]
pub fn start_lock_manager_blocking() -> Result<()> {
    if LOCK_MANAGER
        .lock()
        .expect("LOCK_MANAGER mutex was poisoned")
        .is_some()
    {
        return Ok(());
    }

    eprintln!("Acquiring lock...");
    let token = loop {
        match crate::lock::lock(LOCK_TTL)? {
            Some(t) => {
                eprintln!("Lock acquired.");
                break t;
            }
            None => {
                eprintln!("Failed to acquire lock, retrying in 5s...");
                thread::sleep(LOCK_RENEW_INTERVAL)
            }
        }
    };

    let stop = Arc::new(AtomicBool::new(false));
    // Ctrl+C handler once; best-effort unlock
    if !CTRL_C_INSTALLED.swap(true, Ordering::SeqCst) {
        let stop_for_sig = stop.clone();
        let token_for_sig = token.clone();
        let _ = ctrlc::set_handler(move || {
            eprintln!("Ctrl+C detected, unlocking and exiting.");
            let _ = crate::lock::unlock(&token_for_sig, false);
            stop_for_sig.store(true, Ordering::SeqCst);
            std::process::exit(130);
        });
    }

    let token_clone = token.clone();
    let stop_clone = stop.clone();
    let handle = thread::spawn(move || {
        let mut consecutive_failures = 0u32;
        loop {
            for _ in 0..20 {
                if stop_clone.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            match crate::lock::extend(&token_clone, LOCK_TTL) {
                Ok(true) => consecutive_failures = 0,
                Ok(false) => {
                    eprintln!("Lock extend rejected; exiting immediately.");
                    std::process::exit(1);
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!(
                        "Lock extend error (streak {} / 6): {}",
                        consecutive_failures, e
                    );
                }
            }
            if consecutive_failures >= 6 {
                eprintln!("Lock extend failed 6 times consecutively; exiting process.");
                std::process::exit(1);
            }
        }
    });

    *LOCK_MANAGER.lock().unwrap() = Some(LockRunner {
        stop,
        handle: Some(handle),
        token,
    });
    // Install panic hook and thread-local guard for normal exit.
    install_panic_hook_once();
    NORMAL_EXIT_GUARD.with(|_| {});
    Ok(())
}

/// Stops the lock manager and unlocks.
#[cfg(feature = "reqwest")]
pub fn stop_lock_manager_blocking() {
    let mut mgr = LOCK_MANAGER.lock().unwrap();
    if let Some(mut lr) = mgr.take() {
        eprintln!("Stopping lock manager and unlocking...");
        lr.stop.store(true, Ordering::SeqCst);
        if let Some(h) = lr.handle.take()
            && let Err(e) = h.join()
        {
            eprintln!("Lock renewal thread panicked: {:?}", e);
        } else {
            eprintln!("Lock renewal thread exited cleanly.");
        }
        let _ = crate::lock::unlock(&lr.token, false);
        eprintln!("Unlock complete.");
    }
}
