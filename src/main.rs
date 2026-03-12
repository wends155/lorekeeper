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

    // 2. Discover project root
    let root = match env::var("LOREKEEPER_ROOT") {
        Ok(val) => PathBuf::from(val),
        Err(_) => env::current_dir().map_err(|e| LoreError::ProjectRoot(e.to_string()))?,
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

    let entry_count: i64 = db.conn.query_row("SELECT count(*) FROM entry", [], |row| row.get(0))?;

    info!("Lorekeeper memory database loaded successfully ({} entries)", entry_count);

    // Phase 2 will start the MCP server loop here
    info!("Phase 1: Foundation complete. Exiting cleanly.");

    Ok(())
}
