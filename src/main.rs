//! Main entry point for the Lorekeeper MCP server.
//!
//! This binary provides the standard I/O transport listener for MCP tools.

use lorekeeper::db::Database;
use lorekeeper::error::LoreError;
use lorekeeper::server::LoreHandler;
use lorekeeper::store::sqlite::SqliteEntryRepo;
use rust_mcp_sdk::mcp_server::McpServerOptions;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, ServerCapabilities, ServerCapabilitiesTools,
};
use rust_mcp_sdk::{McpServer, StdioTransport, ToMcpServerHandlerCore, TransportOptions};
use std::sync::Arc;
use std::{env, path::PathBuf};
use tracing::{Level, info};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), LoreError> {
    // 1. Initialize tracing to stderr (stdout reserved for MCP JSON-RPC)
    let filter =
        EnvFilter::try_from_env("LOREKEEPER_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_writer(std::io::stderr).with_max_level(Level::TRACE).init();

    // 2. Discover project root
    let root = if let Ok(val) = env::var("LOREKEEPER_ROOT") {
        PathBuf::from(val)
    } else {
        let cwd = env::current_dir().map_err(|e| LoreError::ProjectRoot(e.to_string()))?;
        find_project_root(&cwd).ok_or_else(|| {
            LoreError::ProjectRoot(
                "could not locate project root — set LOREKEEPER_ROOT or run from within a project directory".to_owned(),
            )
        })?
    };

    info!("Starting Lorekeeper in project root: {}", root.display());

    // 3. Create .lorekeeper directory
    let lorekeeper_dir = root.join(".lorekeeper");
    if !lorekeeper_dir.exists() {
        std::fs::create_dir_all(&lorekeeper_dir)
            .map_err(|e| LoreError::Validation(format!("failed to create .lorekeeper dir: {e}")))?;
        info!("Created .lorekeeper directory");
    }

    // 4. Open Database & build repository
    let db_path = lorekeeper_dir.join("memory.db");
    let db = Database::open(&db_path)?;

    let entry_count: i64 =
        db.connection().query_row("SELECT count(*) FROM entry", [], |row| row.get(0))?;
    info!("Lorekeeper memory database loaded ({} entries)", entry_count);

    // Consume the db to extract the connection for the repo
    let repo = Arc::new(SqliteEntryRepo::new(db.into_connection()));

    // 5. Build MCP server
    let handler = LoreHandler::new(repo);

    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| LoreError::Internal(e.to_string()))?;

    let server_details = InitializeResult {
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: Some(false) }),
            completions: None,
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tasks: None,
        },
        instructions: Some(
            "Lorekeeper: Structured agent memory bank. \
             Use lore_store to save decisions, constraints, lessons, plans, etc. \
             Use lore_search, lore_get, lore_recent, lore_by_type to retrieve. \
             Use lore_stats for an overview."
                .into(),
        ),
        meta: None,
        protocol_version: "2025-03-26".into(),
        server_info: Implementation {
            name: "lorekeeper".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: Some("Structured agent memory bank MCP server".into()),
            icons: vec![],
            title: Some("Lorekeeper".into()),
            website_url: None,
        },
    };

    let server = rust_mcp_sdk::mcp_server::server_runtime_core::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    info!("Lorekeeper MCP server starting on stdio...");
    server.start().await.map_err(|e| LoreError::Internal(e.to_string()))?;

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
