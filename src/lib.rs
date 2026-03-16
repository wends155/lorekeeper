//! Lorekeeper — Agent Long-Term Memory Bank.
//!
//! A [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server that provides
//! structured, queryable memory for AI coding agents. Replaces flat-file history with
//! typed entries persisted in `SQLite` with FTS5 full-text search.
//!
//! # Key Types
//!
//! - [`model::entry::Entry`] — A stored memory entry.
//! - [`model::types::EntryType`] — The 11 semantic entry types (DECISION, COMMIT, etc.).
//! - [`store::repository::EntryRepository`] — Trait for entry persistence.
//! - [`error::LoreError`] — Crate-level error enum.
//! - [`config::LoreConfig`] — Project-local configuration.
//!
//! # Usage
//!
//! The server is started via `main.rs` and communicates over stdio using the MCP protocol.
//! Agents interact with memory through 11 registered tools (store, get, search, etc.).

pub mod config;
pub mod db;
pub mod error;
pub mod model;
pub mod render;
pub mod server;
pub mod store;

