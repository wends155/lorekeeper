//! Markdown rendering for memory entries.

use crate::model::entry::Entry;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Renders a slice of entries into a single Markdown string.
///
/// Entries are grouped by type and sorted by creation date within each group.
pub fn render_entries(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return "No entries found.\n".to_owned();
    }

    let mut grouped = BTreeMap::new();
    for entry in entries {
        grouped.entry(entry.entry_type).or_insert_with(Vec::new).push(entry);
    }

    let mut output = String::new();
    output.push_str("# Lorekeeper Memory Dump\n\n");

    for (et, mut entries) in grouped {
        let _ = writeln!(output, "## {}\n", format!("{et:?}").to_uppercase());

        entries.sort_by_key(|e| e.created_at);

        for entry in entries {
            let _ = writeln!(
                output,
                "### [{}] {}\n",
                format!("{:?}", entry.entry_type).to_uppercase(),
                entry.title
            );
            let _ = writeln!(output, "- **ID:** {}", entry.id.0);
            let _ = writeln!(output, "- **Created:** {}", entry.created_at.to_rfc3339());
            let _ = writeln!(output, "- **Role:** {}", entry.role);
            if !entry.tags.is_empty() {
                let _ = writeln!(output, "- **Tags:** {}", entry.tags.join(", "));
            }
            if !entry.data.is_null() {
                let _ = writeln!(
                    output,
                    "- **Data:**\n```json\n{}\n```",
                    serde_json::to_string_pretty(&entry.data).unwrap_or_default()
                );
            }
            output.push('\n');
            if let Some(body) = &entry.body {
                output.push_str(body);
                output.push_str("\n\n");
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::entry::{Entry, EntryId};
    use crate::model::types::EntryType;
    use chrono::{TimeZone, Utc};

    fn test_entry(id: &str, et: EntryType, title: &str, created_days_ago: i64) -> Entry {
        let now = Utc::now();
        let created = Utc.timestamp_opt(now.timestamp() - created_days_ago * 86400, 0).unwrap();
        Entry {
            id: EntryId(id.into()),
            entry_type: et,
            title: title.into(),
            body: Some("Body text".into()),
            role: "architect".into(),
            tags: vec!["tag1".into()],
            related_entries: vec![],
            created_at: created,
            updated_at: created,
            is_deleted: false,
            access_count: 0,
            last_accessed_at: None,
            data: serde_json::Value::Null,
        }
    }

    #[test]
    fn render_empty_entries() {
        let result = render_entries(&[]);
        assert_eq!(result, "No entries found.\n");
    }

    #[test]
    fn render_single_entry() {
        let entries = vec![test_entry("id1", EntryType::Decision, "Title 1", 0)];
        let result = render_entries(&entries);
        assert!(result.contains("### [DECISION] Title 1"));
        assert!(result.contains("- **ID:** id1"));
    }

    #[test]
    fn render_groups_by_type() {
        let entries = vec![
            test_entry("id1", EntryType::Decision, "D1", 0),
            test_entry("id2", EntryType::Commit, "C1", 0),
        ];
        let result = render_entries(&entries);
        assert!(result.contains("## DECISION"));
        assert!(result.contains("## COMMIT"));
    }

    #[test]
    fn render_entry_with_data() {
        let mut entry = test_entry("id1", EntryType::Plan, "Title 1", 0);
        entry.data = serde_json::json!({
            "status": "planned",
            "scope": "test",
            "tier": "S"
        });
        let entries = vec![entry];
        let result = render_entries(&entries);
        assert!(result.contains("- **Data:**"));
        assert!(result.contains("\"status\": \"planned\""));
        assert!(result.contains("```json"));
    }

    #[test]
    fn render_entry_with_tags_shows_tags_line() {
        let entry = test_entry("id1", EntryType::Decision, "Title 1", 0);
        let entries = vec![entry];
        let result = render_entries(&entries);
        assert!(result.contains("- **Tags:** tag1"));
    }

    #[test]
    fn render_entry_without_tags_omits_tags_line() {
        let mut entry = test_entry("id1", EntryType::Decision, "Title 1", 0);
        entry.tags = vec![];
        let entries = vec![entry];
        let result = render_entries(&entries);
        assert!(!result.contains("- **Tags:**"));
    }
}
