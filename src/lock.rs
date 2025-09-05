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
    sql::exec(
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

    // Verify acquisition by checking the token and that the lock is in the future.
    let verified: Option<String> = sql::cell(
        r#"
        SELECT lock_token
        FROM locks
        WHERE lock_id = 1
          AND lock_token = :lock_token
          AND lock_expired >= CURRENT_TIMESTAMP
        "#,
        params! { "lock_token" => &token },
    )?;

    Ok(verified)
}

/// Release the lock.
/// - If `force` is false: expire only when the token matches and lock is active.
/// - If `force` is true: forcefully expire, set lock_user to current user, and clear token.
pub fn unlock(lock_token: &str, force: bool) -> Result<()> {
    if force {
        let user = current_username();
        sql::exec(
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
        return Ok(());
    }

    // Non-force: expire only if token matches and it's still active (future).
    sql::exec(
        r#"
        UPDATE locks
        SET lock_expired = DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 SECOND)
        WHERE lock_id = 1
          AND lock_token = :lock_token
          AND lock_expired > CURRENT_TIMESTAMP
        "#,
        params! { "lock_token" => lock_token },
    )?;

    Ok(())
}
