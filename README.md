# Lorekeeper

> Agent Long-Term Memory Bank using SQLite and FTS5.

[![CI](https://github.com/wends155/lorekeeper/actions/workflows/ci.yml/badge.svg)](https://github.com/wends155/lorekeeper/actions) [![Crates.io](https://img.shields.io/crates/v/lorekeeper)](https://crates.io/crates/lorekeeper) [![docs.rs](https://img.shields.io/docsrs/lorekeeper)](https://docs.rs/lorekeeper) [![GitHub release](https://img.shields.io/github/v/release/wends155/lorekeeper)](https://github.com/wends155/lorekeeper/releases) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Lorekeeper is a Rust MCP (Model Context Protocol) server that provides structured long-term memory for AI coding agents (operating under the TARS protocol or similar workflows). It replaces flat-file history with a queryable SQLite database, enabling agents to store, search, and retrieve typed memory entries via MCP tools over stdio.

## Key Features

- **Context Window Management:** Reduces per-session token load by allowing selective retrieval.
- **Typed Entries:** 10 semantic entry types (e.g., `DECISION`, `COMMIT`, `PLAN`, `LESSON`, `BUILDER_NOTE`).
- **Role Enforcement:** Mechanically prevents unauthorized writes (e.g., Builder agents cannot assert architectural constraints).
- **Full-Text Search:** Backed by SQLite FTS5 across titles, bodies, and tags.
- **Rich Interaction:** 12 MCP tools covering CRUD, search, config, and memory analytics.
- **Isolated Storage:** Automatically manages a project-local SQLite database (`.lorekeeper/memory.db`).

## Installation

### From Source

Requires Rust 1.94.0+ (Edition 2024).

```sh
# Clone the repository
cargo install --path .
# Alternatively, if just is installed:
just install
```

## MCP Configuration

To use Lorekeeper with MCP-compatible clients, add the server to your configuration. 
Since `cargo install` places the binary in `~/.cargo/bin/`, which should be on your PATH, you can usually use the bare command. If it fails to spawn, use the absolute path to `lorekeeper.exe`.

> **Note:** At session start, agents call `lorekeeper_set_root` to point the server at the active project directory. The `LOREKEEPER_ROOT` environment variable is optional and only needed if the server isn't launched from within a project.

**Antigravity (`mcp_config.json`):**
```json
"lorekeeper": {
  "command": "lorekeeper",
  "args": []
}
```

**Claude Desktop (`claude_desktop_config.json`):**
```json
{
  "mcpServers": {
    "lorekeeper": {
      "command": "lorekeeper"
    }
  }
}
```

## Usage for Agents

Lorekeeper provides the following MCP tools for agentic workflows:

- **Write:** `lorekeeper_store`, `lorekeeper_update`, `lorekeeper_delete`
- **Read:** `lorekeeper_get`, `lorekeeper_search`, `lorekeeper_recent`, `lorekeeper_by_type`, `lorekeeper_render`
- **Health:** `lorekeeper_stats`, `lorekeeper_reflect`
- **Config:** `lorekeeper_set_root` (switch active project root)
- **Meta:** `lorekeeper_help`

Agents can self-discover capabilities by calling `lorekeeper_help`.

## Development

The project uses `just` for automation.

```sh
# Run the verification pipeline (fmt, clippy, test)
just check

# Run tests only
just test

# Build the project
just build
```

The database resides locally at `<PROJECT_ROOT>/.lorekeeper/memory.db`.
