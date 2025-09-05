use anyhow::Result;
use mysql::params;
use std::env;
use std::time::Duration;

use crate::sql;

fn current_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn gen_lock_token() -> String {
    let buf: [u8; 5] = rand::random(); // 40 bits => 10 hex chars
    hex::encode(buf)
}

/// Try to acquire lock_id=1 when it's expired. On success returns the new lock_token.
/// The expiration will be set to now + `ttl` seconds.
pub fn lock(ttl: Duration) -> Result<Option<String>> {
    let user = current_username();
    let token = gen_lock_token();
    let ttl_secs = (ttl.as_secs().min(i64::MAX as u64)) as i64;

    // Attempt to acquire the lock only if it's expired in SQL.
    // Use DATE_ADD with INTERVAL in seconds for portability.
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
        eprintln!("[lock] acquired: token={} ttl_secs={}", token, ttl_secs);
        Ok(Some(token))
    } else {
        // Fetch current holder info for diagnostics.
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
            eprintln!("[lock] busy: could not acquire (no row)");
        }
        Ok(None)
    }
}

/// Extend the lock if the token matches and the lock is still active.
/// Sets lock_expired = CURRENT_TIMESTAMP + ttl seconds.
/// Returns true if extended, false if not (e.g., token mismatch or already expired).
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

/// Release the lock.
/// - If `force` is false: expire only when the token matches and lock is active.
/// - If `force` is true: forcefully expire, set lock_user to current user, and clear token.
pub fn unlock(lock_token: &str, force: bool) -> Result<()> {
    if force {
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

    // Non-force: expire only if token matches and it's still active (future).
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
        "still-active-or-mismatch"
    };
    eprintln!(
        "[unlock] forced=false token={} result={}",
        lock_token, result
    );
    Ok(())
}
