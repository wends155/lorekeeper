//! Main entry point for the Lorekeeper MCP server.
//!
//! This binary provides the standard I/O transport listener for MCP tools.

use lorekeeper::config::LoreConfig;
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
#[allow(clippy::too_many_lines)]
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

    // 5. Load project config (auto-generates defaults if .lorekeeper/config.toml is missing)
    let config = LoreConfig::load(&lorekeeper_dir);
    info!(
        "Loaded config: stale_days={}, similarity_threshold={:.2}",
        config.reflect.stale_days, config.store.similarity_threshold
    );

    // 6. Build MCP server
    let handler = LoreHandler::new(repo, config, Some(root.clone()));

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
            "Lorekeeper is your persistent structured memory bank for the TARS workflow. \
             It survives across sessions and context resets.\n\
             \n\
             SESSION START:\n\
              1. Call lorekeeper_set_root if working in a new project (multi-project support).\n\
              2. Call lorekeeper_stats to see the current state of the memory bank.\n\
              3. Call lorekeeper_reflect to surface stale, dead, or duplicate entries.\n\
              4. Call lorekeeper_recent to load recent context (last 10 entries).\n\
              5. Before making decisions, call lorekeeper_search to check for prior decisions and constraints.\n\
              \n\
              DURING WORK:\n\
              - Architectural decision made → lorekeeper_store with type DECISION (architect role)\n\
              - Git commit completed → lorekeeper_store with type COMMIT (builder role)\n\
              - Constraint discovered → lorekeeper_store with type CONSTRAINT (architect role)\n\
              - Lesson learned from a bug → lorekeeper_store with type LESSON (architect role)\n\
              - Plan created → lorekeeper_store with type PLAN (architect role)\n\
              - Work deferred → lorekeeper_store with type DEFERRED (either role)\n\
              - Technical debt noted → lorekeeper_store with type TECH_DEBT (either role)\n\
              - Stub registered → lorekeeper_store with type STUB (builder role)\n\
              Note: lorekeeper_store returns similar_entries when duplicates are detected.\n\
              \n\
              SESSION END:\n\
              - Summarize session → lorekeeper_store with type SESSION_SUMMARY (either role).\n\
              - Update PLAN entries: set status to executed or abandoned via lorekeeper_update.\n\
              - Resolve completed STUBs: set status to resolved via lorekeeper_update.\n\
              - Call lorekeeper_render to produce a human-readable memory dump if requested.\n\
              \n\
              ROLE ENFORCEMENT:\n\
              Your role field must match your current TARS phase.\n\
              - architect (Think/Reflect phases): DECISION, CONSTRAINT, LESSON, PLAN, FEATURE\n\
              - builder (Act phase): COMMIT, STUB, BUILDER_NOTE\n\
              - both roles: DEFERRED, TECH_DEBT, SESSION_SUMMARY\n\
             \n\
             TAGGING BEST PRACTICES:\n\
             - Use lowercase, descriptive tags: [auth, database, phase-2, breaking-change]\n\
             - Tags are full-text searchable via lorekeeper_search.\n\
             Call lorekeeper_help with a topic for detailed guidance on any entry type or tool."
                .into(),
        ),
        meta: None,
        protocol_version: "2025-11-25".into(),
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
        message_observer: None,
    });

    info!("Lorekeeper MCP server starting on stdio...");

    tokio::select! {
        result = server.start() => {
            if let Err(e) = result {
                return Err(LoreError::Internal(e.to_string()));
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Lorekeeper received SIGINT — shutting down gracefully.");
        }
    }

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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::find_project_root;
    use std::fs;

    #[test]
    fn find_project_root_via_git() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(tmp.path().join(".git")).expect("create .git");
        assert_eq!(find_project_root(tmp.path()), Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn find_project_root_via_lorekeeper_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // Both markers exist — .lorekeeper (Pass 1) must win over .git (Pass 2)
        fs::create_dir(tmp.path().join(".lorekeeper")).expect("create .lorekeeper");
        fs::create_dir(tmp.path().join(".git")).expect("create .git");
        assert_eq!(find_project_root(tmp.path()), Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn find_project_root_returns_none_when_no_markers() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert_eq!(find_project_root(tmp.path()), None);
    }

    #[test]
    fn find_project_root_walks_up_to_git() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(tmp.path().join(".git")).expect("create .git");
        let nested = tmp.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).expect("create nested dirs");
        assert_eq!(find_project_root(&nested), Some(tmp.path().to_path_buf()));
    }
}
