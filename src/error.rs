#[derive(Debug, thiserror::Error)]
pub enum LoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("entry not found: {0}")]
    NotFound(String),

    #[error("role violation: {role} cannot write {entry_type}")]
    RoleViolation { role: String, entry_type: String },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("project root not found: {0}")]
    ProjectRoot(String),
}
