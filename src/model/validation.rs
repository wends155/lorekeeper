//! Validation logic for TARS role enforcement and schema compliance.

use super::{
    entry::{EntryId, NewEntry},
    types::EntryType,
};
use crate::error::LoreError;

/// Validates a new entry against role and type-specific rules.
///
/// This checks:
/// 1. Title is not empty.
/// 2. Role is authorized to write this entry type.
/// 3. Metadata schema matches the entry type.
///
/// # Errors
///
/// Returns [`LoreError`] if validation fails (empty title, wrong role, or bad schema).
pub fn validate_new_entry(entry: &NewEntry) -> Result<(), LoreError> {
    if entry.title.trim().is_empty() {
        return Err(LoreError::Validation("title cannot be empty".to_owned()));
    }
    validate_role(&entry.role, entry.entry_type)?;
    validate_update(entry.entry_type, entry.data.as_ref())?;
    Ok(())
}

/// Validates that a role is authorized to write a specific entry type.
///
/// # Errors
///
/// Returns [`LoreError::RoleViolation`] if the role is not permitted to create the given type.
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
///
/// # Errors
///
/// Returns [`LoreError`] if the `data` field cannot be deserialized into the expected schema.
pub fn validate_update(
    entry_type: EntryType,
    data: Option<&serde_json::Value>,
) -> Result<(), LoreError> {
    use super::types::{
        BuilderNoteData, CommitData, ConstraintData, DeferredData, FeatureData, LessonData,
        PlanData, SessionSummaryData, StubData, TechDebtData,
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
        EntryType::SessionSummary => {
            serde_json::from_value::<SessionSummaryData>(data.clone())?;
        }
    }

    Ok(())
}

/// Validates that a state transition in the `data.status` field is legal.
///
/// State machine rules:
/// - PLAN: `planned` -> `executed` | `planned` -> `abandoned` (no revert allowed)
/// - STUB: `open` -> `resolved` (no revert allowed)
///
/// If either value is `None` the check is skipped.
///
/// # Errors
///
/// Returns [`LoreError::Validation`] when the transition is illegal.
pub fn validate_state_transition(
    entry_type: EntryType,
    current_status: Option<&str>,
    new_status: Option<&str>,
) -> Result<(), LoreError> {
    let (Some(current), Some(next)) = (current_status, new_status) else {
        return Ok(());
    };
    if current == next {
        return Ok(());
    }

    match entry_type {
        EntryType::Plan => {
            let valid = matches!((current, next), ("planned", "executed" | "abandoned"));
            if !valid {
                return Err(LoreError::Validation(format!(
                    "PLAN state transition `{current}` -> `{next}` is not permitted; \
                     allowed: planned->executed, planned->abandoned"
                )));
            }
        }
        EntryType::Stub => {
            let valid = matches!((current, next), ("open", "resolved"));
            if !valid {
                return Err(LoreError::Validation(format!(
                    "STUB state transition `{current}` -> `{next}` is not permitted; \
                     allowed: open->resolved"
                )));
            }
        }
        _ => {} // No state machine for other types
    }

    Ok(())
}

/// Validates that all entries in `related_entries` are valid UUIDs.
///
/// # Errors
///
/// Returns [`LoreError::Validation`] if any string is not a well-formed UUID.
pub fn validate_related_entries(ids: &[EntryId]) -> Result<(), LoreError> {
    for entry_id in ids {
        uuid::Uuid::parse_str(&entry_id.0).map_err(|_| {
            LoreError::Validation(format!("related_entry `{}` is not a valid UUID", entry_id.0))
        })?;
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
        entry.title = String::new();
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

    // ---- State machine tests -------------------------------------------------

    #[test]
    fn plan_transition_planned_to_executed_is_valid() {
        assert!(
            validate_state_transition(EntryType::Plan, Some("planned"), Some("executed")).is_ok()
        );
    }

    #[test]
    fn plan_transition_planned_to_abandoned_is_valid() {
        assert!(
            validate_state_transition(EntryType::Plan, Some("planned"), Some("abandoned")).is_ok()
        );
    }

    #[test]
    fn plan_transition_executed_to_planned_is_invalid() {
        let result = validate_state_transition(EntryType::Plan, Some("executed"), Some("planned"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoreError::Validation(_)));
    }

    #[test]
    fn stub_transition_open_to_resolved_is_valid() {
        assert!(validate_state_transition(EntryType::Stub, Some("open"), Some("resolved")).is_ok());
    }

    #[test]
    fn stub_transition_resolved_to_open_is_invalid() {
        let result = validate_state_transition(EntryType::Stub, Some("resolved"), Some("open"));
        assert!(result.is_err());
    }

    #[test]
    fn state_transition_skipped_when_status_none() {
        assert!(validate_state_transition(EntryType::Plan, None, Some("executed")).is_ok());
        assert!(validate_state_transition(EntryType::Plan, Some("planned"), None).is_ok());
    }

    // ---- UUID validation tests -----------------------------------------------

    #[test]
    fn related_entries_valid_uuids_passes() {
        let ids = vec![
            EntryId("01957ab6-0000-7000-b000-000000000001".to_string()),
            EntryId("01957ab6-0000-7000-b000-000000000002".to_string()),
        ];
        assert!(validate_related_entries(&ids).is_ok());
    }

    #[test]
    fn related_entries_invalid_uuid_fails() {
        let ids = vec![EntryId("not-a-uuid".to_string())];
        let result = validate_related_entries(&ids);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoreError::Validation(_)));
    }

    #[test]
    fn state_transition_same_status_is_noop() {
        assert!(
            validate_state_transition(EntryType::Plan, Some("planned"), Some("planned")).is_ok()
        );
    }

    #[test]
    fn state_transition_other_type_ignores_status() {
        assert!(validate_state_transition(EntryType::Decision, Some("foo"), Some("bar")).is_ok());
    }
}
