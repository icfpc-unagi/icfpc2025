use anyhow::{Result, bail};
use clap::{ArgAction, Parser};

use icfpc2025::lock as lockmod;
use icfpc2025::sql;

#[derive(Parser, Debug)]
#[command(name = "unlock")]
#[command(about = "Release DB lock (lock_id=1)")]
struct Args {
    /// Forcefully release regardless of token
    #[arg(short = 'f', long = "force", action = ArgAction::SetTrue)]
    force: bool,

    /// Lock token (required unless --force)
    token: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Read current state
    let row = match sql::row(
        r#"
        SELECT
            lock_user,
            lock_token,
            (lock_expired > CURRENT_TIMESTAMP) AS active,
            DATE_FORMAT(lock_created, '%Y-%m-%d %H:%i:%s') AS created
        FROM locks
        WHERE lock_id = 1
        "#,
        (),
    )? {
        Some(r) => r,
        None => bail!("locks row not found"),
    };

    let lock_user: String = row.get("lock_user")?;
    let current_token: String = row.get("lock_token")?;
    let active: i64 = row.get("active")?; // 1 or 0
    let created: String = row.get("created")?;

    if args.force {
        lockmod::unlock("", true)?;
        println!("{}", created);
        return Ok(());
    }

    let Some(ref provided) = args.token else {
        bail!("unlock requires a lock_token unless --force is set");
    };

    if active == 0 {
        // Already expired; treat as success.
        println!("{}", created);
        return Ok(());
    }

    if *provided == current_token {
        lockmod::unlock(provided, false)?;
        println!("{}", created);
        return Ok(());
    }

    // Active and token mismatch
    eprintln!("{} is using", lock_user);
    std::process::exit(1);
}
