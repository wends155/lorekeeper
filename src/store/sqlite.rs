//! `SQLite` implementation of the `EntryRepository`.

use crate::error::LoreError;
use crate::model::entry::{Entry, NewEntry, UpdateEntry};
use crate::model::types::EntryType;
use crate::model::validation::{
    validate_new_entry, validate_related_entries, validate_state_transition,
};
use crate::store::repository::{EntryRepository, Filters, MemoryStats, SearchQuery};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, params};
use std::sync::Mutex;
use uuid::Uuid;

/// A thread-safe repository that uses `SQLite` for persistent storage.
pub struct SqliteEntryRepo {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for SqliteEntryRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteEntryRepo").finish_non_exhaustive()
    }
}

impl SqliteEntryRepo {
    /// Creates a new `SqliteEntryRepo` from an existing connection.
    pub const fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }
}

#[allow(clippy::significant_drop_tightening, clippy::redundant_closure_for_method_calls)]
impl EntryRepository for SqliteEntryRepo {
    fn store(&self, mut input: NewEntry) -> Result<Entry, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        // Validation
        input.normalize_tags();
        validate_new_entry(&input)?;
        if let Some(related) = &input.related_entries {
            validate_related_entries(related)?;
        }

        let id = Uuid::now_v7().to_string();
        let now = Utc::now();

        let tags_json = serde_json::to_string(&input.tags.clone().unwrap_or_default())
            .map_err(LoreError::Serialization)?;
        let related_json =
            serde_json::to_string(&input.related_entries.clone().unwrap_or_default())
                .map_err(LoreError::Serialization)?;
        let data_json =
            serde_json::to_string(&input.data.clone().unwrap_or(serde_json::Value::Null))
                .map_err(LoreError::Serialization)?;

        conn.execute(
            "INSERT INTO entry (id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, data)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id,
                input.entry_type,
                input.title,
                input.body,
                input.role,
                tags_json,
                related_json,
                now,
                now,
                data_json,
            ],
        )?;

        Ok(Entry {
            id: crate::model::entry::EntryId(id),
            entry_type: input.entry_type,
            title: input.title,
            body: input.body,
            role: input.role,
            tags: input.tags.unwrap_or_default(),
            related_entries: input.related_entries.unwrap_or_default(),
            created_at: now,
            updated_at: now,
            is_deleted: false,
            data: input.data.unwrap_or(serde_json::Value::Null),
        })
    }

    fn get(&self, id: &str) -> Result<Entry, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data FROM entry WHERE id = ?")?;

        let entry = match stmt.query_row(params![id], map_row) {
            Ok(e) => e,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(LoreError::NotFound(id.to_owned()));
            }
            Err(e) => return Err(LoreError::Database(e)),
        };

        if entry.is_deleted {
            return Err(LoreError::NotFound(id.to_owned()));
        }
        Ok(entry)
    }

    fn update(&self, id: &str, mut update: UpdateEntry) -> Result<Entry, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        // Get existing to merge
        let mut stmt = conn.prepare("SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data FROM entry WHERE id = ?")?;
        let existing = match stmt.query_row(params![id], map_row) {
            Ok(e) => e,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(LoreError::NotFound(id.to_owned()));
            }
            Err(e) => return Err(LoreError::Database(e)),
        };

        if existing.is_deleted {
            return Err(LoreError::NotFound(id.to_owned()));
        }

        // Validate state transition for stateful types (PLAN, STUB)
        let current_status = existing.data.get("status").and_then(|v| v.as_str());
        let new_status =
            update.data.as_ref().and_then(|d| d.get("status")).and_then(|v| v.as_str());
        validate_state_transition(existing.entry_type, current_status, new_status)?;

        update.normalize_tags();
        let now = Utc::now();

        let title = update.title.unwrap_or(existing.title);
        let body = update.body.or(existing.body);
        let tags = update.tags.unwrap_or(existing.tags);
        let related = update.related_entries.unwrap_or(existing.related_entries);
        validate_related_entries(&related)?;
        let data = update.data.unwrap_or(existing.data);

        // Validate merged
        let merged_new = NewEntry {
            entry_type: existing.entry_type,
            title: title.clone(),
            body: body.clone(),
            role: existing.role.clone(),
            tags: Some(tags.clone()),
            related_entries: Some(related.clone()),
            data: Some(data.clone()),
        };
        validate_new_entry(&merged_new)?;

        let tags_json = serde_json::to_string(&tags).map_err(LoreError::Serialization)?;
        let related_json = serde_json::to_string(&related).map_err(LoreError::Serialization)?;
        let data_json = serde_json::to_string(&data).map_err(LoreError::Serialization)?;

        conn.execute(
            "UPDATE entry SET title = ?, body = ?, tags = ?, related_entries = ?, data = ?, updated_at = ? WHERE id = ?",
            params![&title, &body, tags_json, related_json, data_json, now, id],
        )?;

        Ok(Entry {
            id: crate::model::entry::EntryId(id.to_owned()),
            entry_type: existing.entry_type,
            title,
            body,
            role: existing.role,
            tags,
            related_entries: related,
            created_at: existing.created_at,
            updated_at: now,
            is_deleted: false,
            data,
        })
    }

    fn delete(&self, id: &str) -> Result<(), LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let rows = conn.execute("UPDATE entry SET is_deleted = 1 WHERE id = ?", params![id])?;
        if rows == 0 {
            return Err(LoreError::NotFound(id.to_owned()));
        }
        Ok(())
    }

    fn search(&self, query: &SearchQuery) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        let mut sql = "SELECT e.id, e.entry_type, e.title, e.body, e.role, e.tags, e.related_entries, e.created_at, e.updated_at, e.is_deleted, e.data 
                       FROM entry e 
                       JOIN entry_fts f ON e.rowid = f.rowid 
                       WHERE entry_fts MATCH ? AND e.is_deleted = 0".to_owned();

        let mut params_vec: Vec<rusqlite::types::Value> = vec![query.query.clone().into()];

        if let Some(et) = query.entry_type {
            sql.push_str(" AND e.entry_type = ?");
            params_vec.push(serde_json::to_string(&et).unwrap_or_default().replace('"', "").into());
        }

        sql.push_str(" ORDER BY rank LIMIT ?");
        params_vec.push(i64::from(query.limit).into());

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn recent(&self, limit: u32) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data FROM entry WHERE is_deleted = 0 ORDER BY created_at DESC LIMIT ?")?;
        let rows = stmt.query_map(params![limit], map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn by_type(&self, entry_type: EntryType, filters: &Filters) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        let mut sql = "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data FROM entry WHERE entry_type = ? AND is_deleted = 0".to_owned();
        let type_str = serde_json::to_string(&entry_type).unwrap_or_default().replace('"', "");
        let mut params_vec: Vec<rusqlite::types::Value> = vec![type_str.into()];

        if let Some(status) = &filters.status {
            sql.push_str(" AND json_extract(data, '$.status') = ?");
            params_vec.push(status.clone().into());
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");
        params_vec.push(i64::from(filters.limit).into());
        params_vec.push(i64::from(filters.offset).into());

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec), map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn stats(&self) -> Result<MemoryStats, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        let mut stmt =
            conn.prepare("SELECT COUNT(*), MAX(updated_at) FROM entry WHERE is_deleted = 0")?;
        let (total, last_updated): (u64, Option<DateTime<Utc>>) =
            stmt.query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut stmt = conn.prepare(
            "SELECT entry_type, COUNT(*) FROM entry WHERE is_deleted = 0 GROUP BY entry_type",
        )?;
        let type_counts = stmt.query_map([], |row| {
            let type_str: String = row.get(0)?;
            let entry_type: EntryType =
                serde_json::from_str(&format!("\"{type_str}\"")).unwrap_or(EntryType::Stub);
            Ok((entry_type, row.get(1)?))
        })?;

        let mut by_type = Vec::new();
        for tc in type_counts {
            by_type.push(tc?);
        }

        Ok(MemoryStats { total, by_type, last_updated })
    }

    fn render_all(&self) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data FROM entry WHERE is_deleted = 0 ORDER BY entry_type, created_at ASC")?;
        let rows = stmt.query_map([], map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

fn map_row(row: &Row) -> rusqlite::Result<Entry> {
    let tags_json: String = row.get(5)?;
    let related_json: String = row.get(6)?;
    let data_json: String = row.get(10)?;

    Ok(Entry {
        id: crate::model::entry::EntryId(row.get(0)?),
        entry_type: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        role: row.get(4)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        related_entries: serde_json::from_str(&related_json).unwrap_or_default(),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        is_deleted: row.get(9)?,
        data: serde_json::from_str(&data_json).unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::db::Database;
    use crate::model::types::PlanData;

    fn setup_repo() -> SqliteEntryRepo {
        let db = Database::open_in_memory().unwrap();
        SqliteEntryRepo::new(db.into_connection())
    }

    #[test]
    fn store_entry_returns_entry_with_id() {
        let repo = setup_repo();
        let new = NewEntry {
            entry_type: EntryType::Plan,
            title: "Test Plan".into(),
            body: Some("Description".into()),
            role: "architect".into(),
            tags: Some(vec!["tag1".into()]),
            related_entries: None,
            data: Some(
                serde_json::to_value(PlanData {
                    scope: "Phase 2".into(),
                    tier: "L".into(),
                    status: "active".into(),
                })
                .unwrap(),
            ),
        };

        let entry = repo.store(new).unwrap();
        assert!(!entry.id.0.is_empty());
        assert_eq!(entry.title, "Test Plan");
        assert_eq!(entry.tags, vec!["tag1"]);
    }

    #[test]
    fn store_entry_validates_input() {
        let repo = setup_repo();
        let new = NewEntry {
            entry_type: EntryType::Plan,
            title: String::new(), // Invalid: empty title
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        };

        let res = repo.store(new);
        assert!(matches!(res, Err(LoreError::Validation(_))));
    }

    #[test]
    fn get_entry_by_id() {
        let repo = setup_repo();
        let new = NewEntry {
            entry_type: EntryType::Decision,
            title: "D1".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        };
        let stored = repo.store(new).unwrap();
        let fetched = repo.get(&stored.id.0).unwrap();
        assert_eq!(fetched.title, "D1");
    }

    #[test]
    fn get_entry_not_found() {
        let repo = setup_repo();
        let res = repo.get("none");
        assert!(matches!(res, Err(LoreError::NotFound(_))));
    }

    #[test]
    fn update_entry_partial() {
        let repo = setup_repo();
        let stored = repo
            .store(NewEntry {
                entry_type: EntryType::Decision,
                title: "Original".into(),
                body: Some("Old body".into()),
                role: "architect".into(),
                tags: None,
                related_entries: None,
                data: None,
            })
            .unwrap();

        let update = UpdateEntry {
            title: Some("New Title".into()),
            body: None,
            tags: None,
            related_entries: None,
            data: None,
        };

        let updated = repo.update(&stored.id.0, update).unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.body, Some("Old body".into()));
        assert!(updated.updated_at > stored.updated_at);
    }

    #[test]
    fn delete_entry_soft() {
        let repo = setup_repo();
        let stored = repo
            .store(NewEntry {
                entry_type: EntryType::Decision,
                title: "To Delete".into(),
                body: None,
                role: "architect".into(),
                tags: None,
                related_entries: None,
                data: None,
            })
            .unwrap();

        repo.delete(&stored.id.0).unwrap();

        let res = repo.get(&stored.id.0);
        assert!(matches!(res, Err(LoreError::NotFound(_))));
    }

    #[test]
    fn search_fts_title_match() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "Super Unique Title".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();

        let results = repo
            .search(&SearchQuery { query: "Super".into(), entry_type: None, limit: 10 })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Super Unique Title");
    }

    #[test]
    fn search_fts_body_match() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "T1".into(),
            body: Some("The quick brown fox".into()),
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();

        let results = repo
            .search(&SearchQuery { query: "quick".into(), entry_type: None, limit: 10 })
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_fts_tag_match() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "T1".into(),
            body: None,
            role: "architect".into(),
            tags: Some(vec!["experimental".into()]),
            related_entries: None,
            data: None,
        })
        .unwrap();

        let results = repo
            .search(&SearchQuery { query: "experimental".into(), entry_type: None, limit: 10 })
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_filter_by_type() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "Match".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();
        repo.store(NewEntry {
            entry_type: EntryType::Commit,
            title: "Match".into(),
            body: None,
            role: "builder".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();

        let results = repo
            .search(&SearchQuery {
                query: "Match".into(),
                entry_type: Some(EntryType::Decision),
                limit: 10,
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry_type, EntryType::Decision);
    }

    #[test]
    fn recent_returns_ordered() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "First".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "Second".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();

        let recent = repo.recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].title, "Second");
    }

    #[test]
    fn by_type_with_pagination() {
        let repo = setup_repo();
        for i in 1..=5 {
            repo.store(NewEntry {
                entry_type: EntryType::Decision,
                title: format!("D{i}"),
                body: None,
                role: "architect".into(),
                tags: None,
                related_entries: None,
                data: None,
            })
            .unwrap();
        }

        let page1 = repo
            .by_type(EntryType::Decision, &Filters { status: None, limit: 2, offset: 0 })
            .unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = repo
            .by_type(EntryType::Decision, &Filters { status: None, limit: 2, offset: 2 })
            .unwrap();
        assert_eq!(page2.len(), 2);
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[test]
    fn stats_returns_counts() {
        let repo = setup_repo();
        repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "D1".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();
        repo.store(NewEntry {
            entry_type: EntryType::Commit,
            title: "C1".into(),
            body: None,
            role: "builder".into(),
            tags: None,
            related_entries: None,
            data: None,
        })
        .unwrap();

        let stats = repo.stats().unwrap();
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn update_rejects_invalid_plan_transition() {
        let repo = setup_repo();
        let stored = repo
            .store(NewEntry {
                entry_type: EntryType::Plan,
                title: "P1".into(),
                body: None,
                role: "architect".into(),
                tags: None,
                related_entries: None,
                data: Some(serde_json::json!({
                    "scope": "test",
                    "tier": "S",
                    "status": "executed"
                })),
            })
            .unwrap();

        let res = repo.update(
            &stored.id.0,
            UpdateEntry {
                data: Some(serde_json::json!({
                    "scope": "test",
                    "tier": "S",
                    "status": "planned"
                })), // Invalid revert
                ..Default::default()
            },
        );

        assert!(matches!(res, Err(crate::error::LoreError::Validation(_))));
    }

    #[test]
    fn store_rejects_invalid_related_entry_uuid() {
        let repo = setup_repo();
        let res = repo.store(NewEntry {
            entry_type: EntryType::Decision,
            title: "D1".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: Some(vec![crate::model::entry::EntryId("invalid-uuid".into())]),
            data: None,
        });

        assert!(matches!(res, Err(crate::error::LoreError::Validation(_))));
    }
}
