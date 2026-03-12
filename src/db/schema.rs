//! SQL schema definitions and migrations.

use crate::error::LoreError;
use rusqlite::Connection;

/// SQL to create the main memory entry table.
pub const CREATE_ENTRY_TABLE: &str = r"
CREATE TABLE IF NOT EXISTS entry (
    id TEXT PRIMARY KEY,
    entry_type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    role TEXT NOT NULL,
    tags TEXT NOT NULL,
    related_entries TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    data TEXT NOT NULL
);
";

/// SQL to create the FTS5 virtual table for full-text search.
pub const CREATE_FTS_TABLE: &str = r"
CREATE VIRTUAL TABLE IF NOT EXISTS entry_fts USING fts5(
    title,
    body,
    tags_text,
    content='entry',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);
";

/// Trigger to sync FTS5 table on INSERT into `entry`.
pub const CREATE_FTS_INSERT_TRIGGER: &str = r"
CREATE TRIGGER IF NOT EXISTS entry_fts_insert AFTER INSERT ON entry BEGIN
  INSERT INTO entry_fts(rowid, title, body, tags_text)
  VALUES (new.rowid, new.title, COALESCE(new.body, ''), new.tags);
END;
";

/// Trigger to sync FTS5 table on UPDATE of `entry`.
pub const CREATE_FTS_UPDATE_TRIGGER: &str = r"
CREATE TRIGGER IF NOT EXISTS entry_fts_update AFTER UPDATE ON entry BEGIN
  INSERT INTO entry_fts(entry_fts, rowid, title, body, tags_text) 
  VALUES ('delete', old.rowid, old.title, COALESCE(old.body, ''), old.tags);
  INSERT INTO entry_fts(rowid, title, body, tags_text)
  VALUES (new.rowid, new.title, COALESCE(new.body, ''), new.tags);
END;
";

/// Trigger to sync FTS5 table on DELETE from `entry`.
pub const CREATE_FTS_DELETE_TRIGGER: &str = r"
CREATE TRIGGER IF NOT EXISTS entry_fts_delete AFTER DELETE ON entry BEGIN
  INSERT INTO entry_fts(entry_fts, rowid, title, body, tags_text) 
  VALUES ('delete', old.rowid, old.title, COALESCE(old.body, ''), old.tags);
END;
";

/// SQL to create the schema version tracking table.
pub const CREATE_SCHEMA_VERSION_TABLE: &str = r"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);
";

/// Initializes the database schema and performs migrations.
pub fn init_schema(conn: &Connection) -> Result<(), LoreError> {
    let tx = conn.unchecked_transaction()?;

    tx.execute(CREATE_SCHEMA_VERSION_TABLE, [])?;

    let version: i32 = tx
        .query_row("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if version == 0 {
        tx.execute(CREATE_ENTRY_TABLE, [])?;
        tx.execute(CREATE_FTS_TABLE, [])?;
        tx.execute(CREATE_FTS_INSERT_TRIGGER, [])?;
        tx.execute(CREATE_FTS_UPDATE_TRIGGER, [])?;
        tx.execute(CREATE_FTS_DELETE_TRIGGER, [])?;
        tx.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    tx.commit()?;
    Ok(())
}
