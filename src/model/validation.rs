use super::{entry::NewEntry, types::EntryType};
use crate::error::LoreError;

pub fn validate_new_entry(entry: &NewEntry) -> Result<(), LoreError> {
    if entry.title.trim().is_empty() {
        return Err(LoreError::Validation("title cannot be empty".to_string()));
    }
    validate_role(&entry.role, entry.entry_type)?;
    validate_update(entry.entry_type, entry.data.as_ref())?;
    Ok(())
}

pub fn validate_role(role: &str, entry_type: EntryType) -> Result<(), LoreError> {
    let allowed = entry_type.allowed_roles();
    if !allowed.contains(&role.to_lowercase().as_str()) {
        return Err(LoreError::RoleViolation {
            role: role.to_string(),
            entry_type: format!("{entry_type:?}"),
        });
    }
    Ok(())
}

pub fn validate_update(
    entry_type: EntryType,
    data: Option<&serde_json::Value>,
) -> Result<(), LoreError> {
    let data = match data {
        Some(d) => d,
        None => return Ok(()),
    };

    use super::types::{
        BuilderNoteData, CommitData, ConstraintData, DeferredData, FeatureData, LessonData, PlanData,
        StubData, TechDebtData,
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
        entry.title = "".to_string(); // Empty title should fail
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
        assert!(matches!(
            result.unwrap_err(),
            LoreError::RoleViolation { .. }
        ));
    }

    #[test]
    fn validate_role_architect_cannot_write_commit() {
        let result = validate_role("architect", EntryType::Commit);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LoreError::RoleViolation { .. }
        ));
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
}
