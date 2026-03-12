use super::types::EntryType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: EntryId,
    pub entry_type: EntryType,
    pub title: String,
    pub body: Option<String>,
    pub role: String,
    pub tags: Vec<String>,
    pub related_entries: Vec<EntryId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEntry {
    pub entry_type: EntryType,
    pub title: String,
    pub body: Option<String>,
    pub role: String,
    pub tags: Option<Vec<String>>,
    pub related_entries: Option<Vec<EntryId>>,
    pub data: Option<serde_json::Value>,
}

impl NewEntry {
    pub fn normalize_tags(&mut self) {
        if let Some(tags) = &mut self.tags {
            for tag in tags.iter_mut() {
                *tag = tag.trim().to_lowercase();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEntry {
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub related_entries: Option<Vec<EntryId>>,
    pub data: Option<serde_json::Value>,
}

impl UpdateEntry {
    pub fn normalize_tags(&mut self) {
        if let Some(tags) = &mut self.tags {
            for tag in tags.iter_mut() {
                *tag = tag.trim().to_lowercase();
            }
        }
    }
}
