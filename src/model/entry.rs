//! Core memory entry structures.

use super::types::EntryType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A unique identifier for a memory entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub String);

/// A stored memory entry in the Lorekeeper system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Unique identifier for the entry.
    pub id: EntryId,
    /// The TARS-specific type of this entry.
    pub entry_type: EntryType,
    /// Human-readable title of the memory.
    pub title: String,
    /// Detailed content of the memory (optional).
    pub body: Option<String>,
    /// The role (architect/builder) that created this entry.
    pub role: String,
    /// Searchable keywords/tags.
    pub tags: Vec<String>,
    /// IDs of related memory entries for contextual linking.
    pub related_entries: Vec<EntryId>,
    /// When the entry was first stored.
    pub created_at: DateTime<Utc>,
    /// When the entry was last modified.
    pub updated_at: DateTime<Utc>,
    /// Whether the entry is soft-deleted.
    pub is_deleted: bool,
    /// Type-specific structured metadata (variant-dependent).
    pub data: serde_json::Value,
}

/// Parameters for creating a new memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEntry {
    /// The type of entry being created.
    pub entry_type: EntryType,
    /// Human-readable title.
    pub title: String,
    /// Detailed content.
    pub body: Option<String>,
    /// The role of the agent creating the entry.
    pub role: String,
    /// Optional search tags.
    pub tags: Option<Vec<String>>,
    /// Optional related entry links.
    pub related_entries: Option<Vec<EntryId>>,
    /// Optional structured metadata.
    pub data: Option<serde_json::Value>,
}

impl NewEntry {
    /// Normalizes tags by trimming whitespace and converting to lowercase.
    pub fn normalize_tags(&mut self) {
        if let Some(tags) = &mut self.tags {
            for tag in tags.iter_mut() {
                *tag = tag.trim().to_lowercase();
            }
        }
    }
}

/// Parameters for updating an existing memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntry {
    /// New title for the entry.
    pub title: Option<String>,
    /// New body content.
    pub body: Option<String>,
    /// New set of tags.
    pub tags: Option<Vec<String>>,
    /// New set of related entries.
    pub related_entries: Option<Vec<EntryId>>,
    /// New structured metadata.
    pub data: Option<serde_json::Value>,
}

impl UpdateEntry {
    /// Normalizes tags by trimming whitespace and converting to lowercase.
    pub fn normalize_tags(&mut self) {
        if let Some(tags) = &mut self.tags {
            for tag in tags.iter_mut() {
                *tag = tag.trim().to_lowercase();
            }
        }
    }
}
