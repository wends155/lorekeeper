#![allow(clippy::missing_errors_doc)]

//! Lorekeeper MCP Server
//! Agent Long-Term Memory Bank using `SQLite` and `FTS5`.

pub mod db;
pub mod error;
pub mod model;
pub mod render;
pub mod server;
pub mod store;
