//! Persistent storage abstraction and implementations.

pub mod repository;
pub mod sqlite;

pub use repository::*;
pub use sqlite::SqliteEntryRepo;
