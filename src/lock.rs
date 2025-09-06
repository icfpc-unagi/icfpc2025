//! # Distributed Lock Client
//!
//! This module provides a simple distributed locking mechanism using a MySQL backend.
//! It is designed to ensure that only one process can be actively working on a
//! contest problem at a time. The lock is managed via a single row in a `locks`
//! table, identified by `lock_id = 1`.
//!
//! The core functions are:
//! - `lock()`: To acquire the lock if it's available.
//! - `extend()`: To renew the lock's expiration time (heartbeat).
//! - `unlock()`: To release the lock.
//!
//! This implementation is used by the `api` module to manage the lifecycle of
//! a problem-solving session (`select` -> `explore`* -> `guess`).

use anyhow::Result;
use cached::proc_macro::once;
use mysql::params;
use std::env;
use std::time::Duration;

use crate::sql;

/// Gets the current user's name from environment variables (`USER` or `USERNAME`).
/// Used for logging who holds or last held the lock.
#[once]
fn current_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Generates a random 40-bit token represented as a 10-character hex string.
fn gen_lock_token() -> String {
    let buf: [u8; 5] = rand::random();
    hex::encode(buf)
}

/// Tries to acquire the global lock (`lock_id=1`).
///
/// This function attempts to atomically update the lock row, but only if its
/// `lock_expired` timestamp is in the past. If successful, it sets the current
/// user, a new unique lock token, and a new expiration time.
///
/// # Arguments
/// * `ttl` - The `Duration` for which the lock should be valid.
///
/// # Returns
/// * `Ok(Some(String))` containing the new lock token on success.
/// * `Ok(None)` if the lock is currently held by another process.
/// * `Err` if a database error occurs.
pub fn lock(ttl: Duration) -> Result<Option<String>> {
    let user = current_username();
    let token = gen_lock_token();
    let ttl_secs = (ttl.as_secs().min(i64::MAX as u64)) as i64;

    // Attempt to acquire the lock only if it's expired.
    // This is an atomic "test-and-set" operation performed by the database.
    let affected = sql::exec(
        r#"
        UPDATE locks
        SET
            lock_user = :lock_user,
            lock_token = :lock_token,
            lock_expired = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL :ttl SECOND)
        WHERE lock_id = 1 AND lock_expired < CURRENT_TIMESTAMP
        "#,
        params! { "lock_user" => &user, "lock_token" => &token, "ttl" => ttl_secs },
    )?;

    if affected > 0 {
        // We successfully acquired the lock.
        eprintln!("[lock] acquired: token={} ttl_secs={}", token, ttl_secs);
        Ok(Some(token))
    } else {
        // The lock is currently held by someone else. Fetch info for logging.
        let info = sql::row(
            r#"
            SELECT lock_user,
                   DATE_FORMAT(lock_expired, '%Y-%m-%d %H:%i:%s') AS lock_expired
            FROM locks WHERE lock_id = 1
            "#,
            (),
        )
        .ok()
        .flatten();
        if let Some(r) = info {
            let user: Option<String> = r.get_option("lock_user").unwrap_or(None);
            let exp: Option<String> = r.get_option("lock_expired").unwrap_or(None);
            eprintln!(
                "[lock] busy: could not acquire (user={:?}, expires={:?})",
                user, exp
            );
        } else {
            eprintln!("[lock] busy: could not acquire (lock row may not exist)");
        }
        Ok(None)
    }
}

/// Extends the expiration time of an active lock.
///
/// This acts as a heartbeat, preventing a valid lock from expiring while the
/// owning process is still working. It will only succeed if the provided `lock_token`
/// matches the one in the database and the lock has not already expired.
///
/// # Arguments
/// * `lock_token` - The token proving ownership of the lock.
/// * `ttl` - The `Duration` to extend the lock's validity from the current time.
///
/// # Returns
/// * `Ok(true)` if the lock was successfully extended.
/// * `Ok(false)` if the lock could not be extended (e.g., token mismatch or expired).
/// * `Err` if a database error occurs.
pub fn extend(lock_token: &str, ttl: Duration) -> Result<bool> {
    let ttl_secs = (ttl.as_secs().min(i64::MAX as u64)) as i64;
    let affected = sql::exec(
        r#"
        UPDATE locks
        SET lock_expired = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL :ttl SECOND)
        WHERE lock_id = 1
          AND lock_token = :lock_token
          AND lock_expired > CURRENT_TIMESTAMP
        "#,
        params! { "ttl" => ttl_secs, "lock_token" => lock_token },
    )?;
    Ok(affected > 0)
}

/// Releases the global lock.
///
/// This function can operate in two modes:
/// - Normal (`force = false`): Releases the lock only if the `lock_token` matches
///   and the lock is still active. This is the standard, safe way to unlock.
/// - Forced (`force = true`): Forcefully expires the lock, regardless of who owns
///   it. This is a recovery mechanism for situations where a lock might be stuck.
///
/// # Arguments
/// * `lock_token` - The token for the lock. In a forced unlock, this is only for logging.
/// * `force` - Whether to perform a forced unlock.
pub fn unlock(lock_token: &str, force: bool) -> Result<()> {
    if force {
        // Forcefully expire the lock and clear the token.
        let user = current_username();
        let affected = sql::exec(
            r#"
            UPDATE locks
            SET
                lock_user = :lock_user,
                lock_token = '',
                lock_expired = DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 SECOND)
            WHERE lock_id = 1
            "#,
            params! { "lock_user" => &user },
        )?;
        let result = if affected > 0 { "expired" } else { "unknown" };
        eprintln!(
            "[unlock] forced=true token={} result={}",
            lock_token, result
        );
        return Ok(());
    }

    // Non-forced: expire the lock only if the token matches and it's still active.
    let affected = sql::exec(
        r#"
        UPDATE locks
        SET lock_expired = DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 SECOND)
        WHERE lock_id = 1
          AND lock_token = :lock_token
          AND lock_expired > CURRENT_TIMESTAMP
        "#,
        params! { "lock_token" => lock_token },
    )?;
    let result = if affected > 0 {
        "expired"
    } else {
        // This could mean the token was wrong, or the lock had already expired.
        "still-active-or-mismatch"
    };
    eprintln!(
        "[unlock] forced=false token={} result={}",
        lock_token, result
    );
    Ok(())
}
