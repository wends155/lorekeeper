#![allow(missing_docs)]

use lorekeeper::{db::Database, error::LoreError};
use std::{env, path::PathBuf};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), LoreError> {
    // 1. Initialize tracing to stderr (stdout reserved for MCP)
    let filter =
        EnvFilter::try_from_env("LOREKEEPER_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_writer(std::io::stderr).with_max_level(Level::TRACE).init();

    // 2. Discover project root per architecture.md §Project Root Discovery:
    //    a) LOREKEEPER_ROOT env var
    //    b) Walk up from CWD looking for .lorekeeper/
    //    c) Walk up from CWD looking for .git/
    //    d) Fail with LoreError::ProjectRoot
    let root = if let Ok(val) = env::var("LOREKEEPER_ROOT") {
        PathBuf::from(val)
    } else {
        let cwd = env::current_dir().map_err(|e| LoreError::ProjectRoot(e.to_string()))?;
        find_project_root(&cwd)
            .ok_or_else(|| LoreError::ProjectRoot("could not locate project root — set LOREKEEPER_ROOT or run from within a project directory".to_owned()))?
    };

    info!("Starting Lorekeeper in project root: {}", root.display());

    // 3. Create .lorekeeper directory
    let lorekeeper_dir = root.join(".lorekeeper");
    if !lorekeeper_dir.exists() {
        std::fs::create_dir_all(&lorekeeper_dir)
            .map_err(|e| LoreError::Validation(format!("failed to create .lorekeeper dir: {e}")))?;
        info!("Created .lorekeeper directory");
    }

    // 4. Open Database
    let db_path = lorekeeper_dir.join("memory.db");
    let db = Database::open(&db_path)?;

    let entry_count: i64 =
        db.connection().query_row("SELECT count(*) FROM entry", [], |row| row.get(0))?;

    info!("Lorekeeper memory database loaded successfully ({} entries)", entry_count);

    // Phase 2 will start the MCP server loop here
    info!("Phase 1: Foundation complete. Exiting cleanly.");

    Ok(())
}

/// Walk up the directory tree from `start`, looking first for a `.lorekeeper/` directory,
/// then for a `.git/` directory. Returns the directory containing the marker if found.
fn find_project_root(start: &std::path::Path) -> Option<PathBuf> {
    // Pass 1: look for .lorekeeper
    let mut dir = start;
    loop {
        if dir.join(".lorekeeper").is_dir() {
            return Some(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    // Pass 2: look for .git
    let mut dir = start;
    loop {
        if dir.join(".git").is_dir() {
            return Some(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    None
}
