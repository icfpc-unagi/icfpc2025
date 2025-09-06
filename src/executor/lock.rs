use anyhow::Result;
use mysql::params;
use std::time::Duration;

use crate::sql;

/// Acquire a per-task lock by setting `task_locked` to now + 30s and writing `task_lock`.
/// Returns true if the lock was acquired.
pub fn acquire_lock(task_id: i64, task_lock: &str) -> Result<bool> {
    let affected = sql::exec(
        r#"
        UPDATE tasks
        SET task_locked = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 SECOND),
            task_lock = :task_lock
        WHERE task_id = :task_id
        "#,
        params! { "task_id" => task_id, "task_lock" => task_lock },
    )?;
    Ok(affected > 0)
}

/// Extends the lock if `task_lock` matches and `task_locked` is still in the future.
pub fn extend_lock(task_id: i64, task_lock: &str) -> Result<bool> {
    let affected = sql::exec(
        r#"
        UPDATE tasks
        SET task_locked = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL 30 SECOND)
        WHERE task_id = :task_id
          AND task_lock = :task_lock
          AND task_locked > CURRENT_TIMESTAMP
        "#,
        params! { "task_id" => task_id, "task_lock" => task_lock },
    )?;
    Ok(affected > 0)
}

/// Releases the lock by setting `task_locked` to NULL if conditions match.
pub fn release_lock(task_id: i64, task_lock: &str) -> Result<bool> {
    let affected = sql::exec(
        r#"
        UPDATE tasks
        SET task_locked = NULL
        WHERE task_id = :task_id
          AND task_lock = :task_lock
          AND task_locked > CURRENT_TIMESTAMP
        "#,
        params! { "task_id" => task_id, "task_lock" => task_lock },
    )?;
    Ok(affected > 0)
}

#[allow(dead_code)]
fn _secs(d: Duration) -> i64 {
    (d.as_secs().min(i64::MAX as u64)) as i64
}
