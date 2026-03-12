//! Traits and types for entry storage abstraction.

use crate::error::LoreError;
use crate::model::entry::{Entry, NewEntry, UpdateEntry};
use crate::model::types::EntryType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parameters for searching memory entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search keywords for FTS5.
    pub query: String,
    /// Optional filter by entry type.
    pub entry_type: Option<EntryType>,
    /// Maximum number of results to return.
    pub limit: u32,
}

/// Pagination and filtering parameters for list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
    /// Optional status filter (stored in `data` JSON).
    pub status: Option<String>,
    /// Maximum number of results to return.
    pub limit: u32,
    /// Number of results to skip.
    pub offset: u32,
}

/// Statistical overview of the memory bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total number of non-deleted entries.
    pub total: u64,
    /// Count of entries grouped by type.
    pub by_type: Vec<(EntryType, u64)>,
    /// Timestamp of the most recent update.
    pub last_updated: Option<DateTime<Utc>>,
}

/// Interface for memory entry persistence.
#[cfg_attr(test, mockall::automock)]
pub trait EntryRepository: Send + Sync {
    /// Stores a new entry and returns the persisted version with its ID.
    fn store(&self, input: NewEntry) -> Result<Entry, LoreError>;
    /// Retrieves an entry by its ID.
    fn get(&self, id: &str) -> Result<Entry, LoreError>;
    /// Updates an existing entry with new fields.
    fn update(&self, id: &str, update: UpdateEntry) -> Result<Entry, LoreError>;
    /// Performs a soft delete on an entry.
    fn delete(&self, id: &str) -> Result<(), LoreError>;
    /// Searches entries using full-text search.
    fn search(&self, query: &SearchQuery) -> Result<Vec<Entry>, LoreError>;
    /// Retrieves the most recently created entries.
    fn recent(&self, limit: u32) -> Result<Vec<Entry>, LoreError>;
    /// Lists entries of a specific type with filters and pagination.
    fn by_type(&self, entry_type: EntryType, filters: &Filters) -> Result<Vec<Entry>, LoreError>;
    /// Gets database statistics.
    fn stats(&self) -> Result<MemoryStats, LoreError>;
    /// Returns all entries ordered for rendering (internal use).
    fn render_all(&self) -> Result<Vec<Entry>, LoreError>;
}
