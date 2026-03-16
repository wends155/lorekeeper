//! `SQLite` implementation of the `EntryRepository`.

use crate::config::LoreConfig;
use crate::error::LoreError;
use crate::model::entry::{Entry, NewEntry, UpdateEntry};
use crate::model::types::{EntryType, ReflectCriteria, ReflectReport, SimilarEntry};
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
            access_count: 0,
            last_accessed_at: None,
            data: input.data.unwrap_or(serde_json::Value::Null),
        })
    }

    fn get(&self, id: &str) -> Result<Entry, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data, access_count, last_accessed_at FROM entry WHERE id = ?",
        )?;

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

        // Track deliberate access (only on explicit get, not on searches)
        let now = Utc::now();
        conn.execute(
            "UPDATE entry SET access_count = access_count + 1, last_accessed_at = ? WHERE id = ?",
            params![now, id],
        )?;

        Ok(entry)
    }

    fn update(&self, id: &str, mut update: UpdateEntry) -> Result<Entry, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        // Get existing to merge
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data, access_count, last_accessed_at FROM entry WHERE id = ?",
        )?;
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
            access_count: existing.access_count,
            last_accessed_at: existing.last_accessed_at,
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
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data, access_count, last_accessed_at FROM entry WHERE is_deleted = 0 ORDER BY created_at DESC LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit], map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn by_type(&self, entry_type: EntryType, filters: &Filters) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        let mut sql = "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data, access_count, last_accessed_at FROM entry WHERE entry_type = ? AND is_deleted = 0".to_owned();
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

        let mut stmt = conn.prepare(
            "SELECT json_extract(data, '$.status'), COUNT(*) \
             FROM entry \
             WHERE is_deleted = 0 AND json_extract(data, '$.status') IS NOT NULL \
             GROUP BY json_extract(data, '$.status')",
        )?;
        let status_counts = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut by_status = Vec::new();
        for sc in status_counts {
            by_status.push(sc?);
        }

        Ok(MemoryStats { total, by_type, by_status, last_updated })
    }

    fn render_all(&self) -> Result<Vec<Entry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT id, entry_type, title, body, role, tags, related_entries, created_at, updated_at, is_deleted, data, access_count, last_accessed_at FROM entry WHERE is_deleted = 0 ORDER BY entry_type, created_at ASC",
        )?;

        let rows = stmt.query_map([], map_row)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn find_similar(
        &self,
        title: &str,
        body: Option<String>,
        entry_type: EntryType,
        threshold: f64,
    ) -> Result<Vec<SimilarEntry>, LoreError> {
        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;

        // Build query string combining title and body for richer FTS5 matching
        let query_text = match body.as_deref() {
            Some(b) if !b.is_empty() => format!("{title} {b}"),
            _ => title.to_owned(),
        };

        // Sanitize for FTS5: remove special characters that break the query parser
        let sanitized: String = query_text
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == ' ' { c } else { ' ' })
            .collect();
        let sanitized = sanitized.trim();
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let type_str =
            serde_json::to_string(&entry_type).map_err(LoreError::Serialization)?.replace('"', "");

        // BM25 score in FTS5 is negative (more negative = more similar)
        let mut stmt = conn.prepare(
            "SELECT e.id, e.title, e.entry_type, rank \
             FROM entry_fts \
             JOIN entry e ON e.rowid = entry_fts.rowid \
             WHERE entry_fts MATCH ? AND e.entry_type = ? AND e.is_deleted = 0 \
             ORDER BY rank \
             LIMIT 3",
        )?;

        let rows = stmt.query_map(rusqlite::params![sanitized, type_str], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (id, row_title, et, score) = row?;
            // BM25 scores are negative; convert to positive for comparison
            let abs_score = score.abs();
            if abs_score >= threshold {
                results.push(SimilarEntry { id, title: row_title, entry_type: et, score });
            }
        }
        Ok(results)
    }

    #[allow(clippy::too_many_lines)]
    fn reflect(
        &self,
        criteria: &ReflectCriteria,
        config: &LoreConfig,
    ) -> Result<ReflectReport, LoreError> {
        use crate::model::types::{MemoryState, ReflectFinding, ReflectFocus, ReflectSummary};

        let conn = self.conn.lock().map_err(|e| LoreError::Poison(e.to_string()))?;
        let limit = i64::from(criteria.limit.unwrap_or(20));

        // Determine memory state
        let total_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM entry WHERE is_deleted = 0", [], |row| {
                row.get(0)
            })?;

        let state = match total_count {
            0 => MemoryState::Empty,
            1..=4 => MemoryState::Nascent,
            5..=99 => MemoryState::Active,
            _ => MemoryState::Mature,
        };

        let guidance = match &state {
            MemoryState::Empty => Some(
                "No entries yet. Store your first memory with lorekeeper_store to get started."
                    .to_owned(),
            ),
            MemoryState::Nascent => Some(
                "Memory bank is nascent (<5 entries). Results may not be representative yet."
                    .to_owned(),
            ),
            _ => None,
        };

        let stale_days = i64::from(criteria.stale_days.unwrap_or(config.reflect.stale_days));
        let hot_threshold =
            i64::from(criteria.min_access_count.unwrap_or(config.reflect.hot_access_threshold));
        let dead_days = i64::from(config.reflect.dead_entry_days);

        let mut findings: Vec<ReflectFinding> = Vec::new();
        let mut summary = ReflectSummary::default();

        let run_stale = matches!(criteria.focus, ReflectFocus::Stale | ReflectFocus::All);
        let run_dead = matches!(criteria.focus, ReflectFocus::Dead | ReflectFocus::All);
        let run_hot = matches!(criteria.focus, ReflectFocus::Hot | ReflectFocus::All);
        let run_orphaned = matches!(criteria.focus, ReflectFocus::Orphaned | ReflectFocus::All);
        let run_contradictions =
            matches!(criteria.focus, ReflectFocus::Contradictions | ReflectFocus::All);

        // Stale: entries not updated within stale_days
        if run_stale {
            let mut stmt = conn.prepare(
                "SELECT id, entry_type, title, updated_at FROM entry \
                 WHERE is_deleted = 0 \
                 AND CAST(julianday('now') - julianday(updated_at) AS INTEGER) >= ? \
                 ORDER BY updated_at ASC \
                 LIMIT ?",
            )?;
            let rows = stmt.query_map(rusqlite::params![stale_days, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (id, et, title, updated_at) = row?;
                summary.stale += 1;
                findings.push(ReflectFinding {
                    category: "stale".to_owned(),
                    entry_id: id,
                    entry_type: et,
                    title,
                    reason: format!("Not updated since {updated_at} (>{stale_days} days)"),
                });
            }
        }

        // Dead: entries with access_count = 0 and older than dead_days
        if run_dead {
            let mut stmt = conn.prepare(
                "SELECT id, entry_type, title, created_at FROM entry \
                 WHERE is_deleted = 0 AND access_count = 0 \
                 AND CAST(julianday('now') - julianday(created_at) AS INTEGER) >= ? \
                 ORDER BY created_at ASC \
                 LIMIT ?",
            )?;
            let rows = stmt.query_map(rusqlite::params![dead_days, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (id, et, title, created_at) = row?;
                summary.dead += 1;
                findings.push(ReflectFinding {
                    category: "dead".to_owned(),
                    entry_id: id,
                    entry_type: et,
                    title,
                    reason: format!("Never accessed since creation ({created_at})"),
                });
            }
        }

        // Hot: frequently accessed entries (may need review/split)
        if run_hot {
            let mut stmt = conn.prepare(
                "SELECT id, entry_type, title, access_count FROM entry \
                 WHERE is_deleted = 0 AND access_count >= ? \
                 ORDER BY access_count DESC \
                 LIMIT ?",
            )?;
            let rows = stmt.query_map(rusqlite::params![hot_threshold, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (id, et, title, count) = row?;
                summary.hot += 1;
                findings.push(ReflectFinding {
                    category: "hot".to_owned(),
                    entry_id: id,
                    entry_type: et,
                    title,
                    reason: format!("Accessed {count} times — consider reviewing for freshness"),
                });
            }
        }

        // Orphaned: entries referencing non-existent or deleted related_entries
        if run_orphaned {
            let mut stmt = conn.prepare(
                "SELECT e.id, e.entry_type, e.title, ref.value FROM entry e, \
                 json_each(e.related_entries) AS ref \
                 WHERE e.is_deleted = 0 \
                 AND NOT EXISTS (SELECT 1 FROM entry r WHERE r.id = ref.value AND r.is_deleted = 0) \
                 LIMIT ?",
            )?;
            let rows = stmt.query_map(rusqlite::params![limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for row in rows {
                let (id, et, title, broken_ref) = row?;
                summary.orphaned += 1;
                findings.push(ReflectFinding {
                    category: "orphaned".to_owned(),
                    entry_id: id,
                    entry_type: et,
                    title,
                    reason: format!("Related entry {broken_ref} no longer exists"),
                });
            }
        }

        // Contradictions: same-type entries with high FTS5 similarity
        if run_contradictions {
            let mut stmt = conn.prepare(
                "SELECT a.id, a.entry_type, a.title, b.title FROM entry a \
                 JOIN entry_fts fa ON fa.rowid = a.rowid \
                 JOIN entry_fts(fa.title || ' ' || COALESCE(fa.body, '')) fb ON TRUE \
                 JOIN entry b ON b.rowid = fb.rowid \
                 WHERE a.is_deleted = 0 AND b.is_deleted = 0 \
                 AND a.entry_type = b.entry_type \
                 AND a.id != b.id \
                 AND fb.rank < -0.5 \
                 LIMIT ?",
            )?;
            let rows = stmt.query_map(rusqlite::params![limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for (id, et, title, similar_title) in rows.flatten() {
                summary.contradictions += 1;
                findings.push(ReflectFinding {
                    category: "contradictions".to_owned(),
                    entry_id: id,
                    entry_type: et,
                    title,
                    reason: format!("Textually similar to: \"{similar_title}\""),
                });
            }
        }

        summary.total = findings.len();

        Ok(ReflectReport { state, findings, summary, guidance })
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
        access_count: row.get(11).unwrap_or(0),
        last_accessed_at: row.get(12).unwrap_or(None),
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
    fn stats_returns_status_breakdown() {
        let repo = setup_repo();
        // Store a planned PLAN
        repo.store(NewEntry {
            entry_type: EntryType::Plan,
            title: "P1".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({ "scope": "s", "tier": "S", "status": "planned" })),
        })
        .unwrap();
        // Store a planned PLAN (second one — same status to check count)
        repo.store(NewEntry {
            entry_type: EntryType::Plan,
            title: "P2".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({ "scope": "s", "tier": "S", "status": "planned" })),
        })
        .unwrap();
        // Store a STUB with a different status
        repo.store(NewEntry {
            entry_type: EntryType::Stub,
            title: "S1".into(),
            body: None,
            role: "builder".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({
                "phase_number": 1,
                "contract": "c",
                "module": "m",
                "status": "open"
            })),
        })
        .unwrap();

        let stats = repo.stats().unwrap();
        assert_eq!(stats.total, 3);

        // Find the counts in by_status
        let planned_count =
            stats.by_status.iter().find(|(s, _)| s == "planned").map_or(0, |(_, n)| *n);
        let open_count = stats.by_status.iter().find(|(s, _)| s == "open").map_or(0, |(_, n)| *n);

        assert_eq!(planned_count, 2, "expected 2 PLANs with status=planned");
        assert_eq!(open_count, 1, "expected 1 STUB with status=open");
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

    #[test]
    fn update_deleted_entry_returns_not_found() {
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

        let res = repo.update(&stored.id.0, UpdateEntry::default());
        assert!(matches!(res, Err(LoreError::NotFound(_))));
    }

    #[test]
    fn by_type_with_status_filter() {
        let repo = setup_repo();
        // Store 2 planned and 1 executed
        repo.store(NewEntry {
            entry_type: EntryType::Plan,
            title: "P1".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({ "scope": "s", "tier": "S", "status": "planned" })),
        })
        .unwrap();
        repo.store(NewEntry {
            entry_type: EntryType::Plan,
            title: "P2".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({ "scope": "s", "tier": "S", "status": "planned" })),
        })
        .unwrap();
        repo.store(NewEntry {
            entry_type: EntryType::Plan,
            title: "P3".into(),
            body: None,
            role: "architect".into(),
            tags: None,
            related_entries: None,
            data: Some(serde_json::json!({ "scope": "s", "tier": "S", "status": "executed" })),
        })
        .unwrap();

        let filtered = repo
            .by_type(
                EntryType::Plan,
                &Filters { status: Some("planned".into()), limit: 10, offset: 0 },
            )
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn search_returns_empty_for_no_match() {
        let repo = setup_repo();
        let results = repo
            .search(&SearchQuery { query: "nonexistent".into(), entry_type: None, limit: 10 })
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn stats_empty_database() {
        let repo = setup_repo();
        let stats = repo.stats().unwrap();
        assert_eq!(stats.total, 0);
        assert!(stats.by_type.is_empty());
        assert!(stats.last_updated.is_none());
    }

    #[test]
    fn poisoned_mutex_returns_error() {
        let err = LoreError::Poison("mutex poisoned".into());
        assert!(err.to_string().contains("mutex poisoned"));
    }
}
