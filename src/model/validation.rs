//! Validation logic for TARS role enforcement and schema compliance.

use super::{entry::NewEntry, types::EntryType};
use crate::error::LoreError;

/// Validates a new entry against role and type-specific rules.
///
/// This checks:
/// 1. Title is not empty.
/// 2. Role is authorized to write this entry type.
/// 3. Metadata schema matches the entry type.
pub fn validate_new_entry(entry: &NewEntry) -> Result<(), LoreError> {
    if entry.title.trim().is_empty() {
        return Err(LoreError::Validation("title cannot be empty".to_owned()));
    }
    validate_role(&entry.role, entry.entry_type)?;
    validate_update(entry.entry_type, entry.data.as_ref())?;
    Ok(())
}

/// Validates that a role is authorized to write a specific entry type.
pub fn validate_role(role: &str, entry_type: EntryType) -> Result<(), LoreError> {
    let allowed = entry_type.allowed_roles();
    if !allowed.contains(&role.to_lowercase().as_str()) {
        return Err(LoreError::RoleViolation {
            role: role.to_owned(),
            entry_type: format!("{entry_type:?}"),
        });
    }
    Ok(())
}

/// Validates the JSON metadata for an entry against its specific Rust type.
pub fn validate_update(
    entry_type: EntryType,
    data: Option<&serde_json::Value>,
) -> Result<(), LoreError> {
    use super::types::{
        BuilderNoteData, CommitData, ConstraintData, DeferredData, FeatureData, LessonData,
        PlanData, StubData, TechDebtData,
    };

    let Some(data) = data else {
        return Ok(());
    };

    match entry_type {
        EntryType::Decision => {} // No required schema
        EntryType::Commit => {
            serde_json::from_value::<CommitData>(data.clone())?;
        }
        EntryType::Constraint => {
            serde_json::from_value::<ConstraintData>(data.clone())?;
        }
        EntryType::Lesson => {
            serde_json::from_value::<LessonData>(data.clone())?;
        }
        EntryType::Plan => {
            serde_json::from_value::<PlanData>(data.clone())?;
        }
        EntryType::Feature => {
            serde_json::from_value::<FeatureData>(data.clone())?;
        }
        EntryType::Stub => {
            serde_json::from_value::<StubData>(data.clone())?;
        }
        EntryType::Deferred => {
            serde_json::from_value::<DeferredData>(data.clone())?;
        }
        EntryType::BuilderNote => {
            serde_json::from_value::<BuilderNoteData>(data.clone())?;
        }
        EntryType::TechDebt => {
            serde_json::from_value::<TechDebtData>(data.clone())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::str_to_string)]

    use super::*;

    fn create_test_entry(entry_type: EntryType, data: Option<serde_json::Value>) -> NewEntry {
        NewEntry {
            entry_type,
            title: "Test".to_string(),
            body: None,
            role: "architect".to_string(),
            tags: None,
            related_entries: None,
            data,
        }
    }

    #[test]
    fn validate_new_entry_happy_path() {
        let entry = create_test_entry(EntryType::Decision, None);
        assert!(validate_new_entry(&entry).is_ok());
    }

    #[test]
    fn validate_new_entry_missing_title() {
        let mut entry = create_test_entry(EntryType::Decision, None);
        entry.title = String::new(); // Empty title should fail
        let result = validate_new_entry(&entry);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoreError::Validation(_)));
    }

    #[test]
    fn validate_new_entry_plan_missing_status() {
        let entry = create_test_entry(
            EntryType::Plan,
            Some(serde_json::json!({ "scope": "test", "tier": "S" })),
        );
        let result = validate_new_entry(&entry);
        assert!(result.is_err());
    }

    #[test]
    fn validate_role_builder_cannot_write_decision() {
        let result = validate_role("builder", EntryType::Decision);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoreError::RoleViolation { .. }));
    }

    #[test]
    fn validate_role_architect_cannot_write_commit() {
        let result = validate_role("architect", EntryType::Commit);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoreError::RoleViolation { .. }));
    }

    #[test]
    fn validate_role_both_can_write_deferred() {
        assert!(validate_role("architect", EntryType::Deferred).is_ok());
        assert!(validate_role("builder", EntryType::Deferred).is_ok());
    }

    #[test]
    fn tag_normalization_lowercases_and_trims() {
        let mut entry = create_test_entry(EntryType::Decision, None);
        entry.tags = Some(vec!["  Rust  ".to_string(), "MCP".to_string()]);
        entry.normalize_tags();
        assert_eq!(entry.tags.unwrap(), vec!["rust", "mcp"]);
    }

    #[test]
    fn validate_new_entry_all_types_happy_path() {
        use serde_json::json;

        let cases: &[(EntryType, &str, Option<serde_json::Value>)] = &[
            (EntryType::Decision, "architect", None),
            (
                EntryType::Commit,
                "builder",
                Some(json!({ "hash": "abc123", "files": ["src/main.rs"] })),
            ),
            (EntryType::Constraint, "architect", Some(json!({ "source": "legal" }))),
            (EntryType::Lesson, "architect", Some(json!({ "root_cause": "misconfig" }))),
            (
                EntryType::Plan,
                "architect",
                Some(json!({ "scope": "module", "tier": "S", "status": "draft" })),
            ),
            (EntryType::Feature, "architect", Some(json!({ "status": "proposed" }))),
            (
                EntryType::Stub,
                "builder",
                Some(
                    json!({ "phase_number": 2, "contract": "trait X", "module": "store", "status": "pending" }),
                ),
            ),
            (
                EntryType::Deferred,
                "architect",
                Some(json!({ "reason": "out of scope", "target_phase": 2 })),
            ),
            (
                EntryType::BuilderNote,
                "builder",
                Some(json!({ "note_type": "observation", "step_ref": "5", "plan_ref": "plan-v1" })),
            ),
            (EntryType::TechDebt, "builder", Some(json!({ "severity": "low", "origin_phase": 1 }))),
        ];

        for (entry_type, role, data) in cases {
            let mut entry = create_test_entry(*entry_type, data.clone());
            entry.role = (*role).to_string();
            let result = validate_new_entry(&entry);
            assert!(
                result.is_ok(),
                "Expected Ok for {entry_type:?} with role {role}, got: {result:?}"
            );
        }
    }
}
