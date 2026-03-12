//! Error types for the Lorekeeper MCP server.

/// The primary error type for Lorekeeper operations.
#[derive(Debug, thiserror::Error)]
pub enum LoreError {
    /// Wraps errors from the underlying `SQLite` database.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Indicates that input validation failed (e.g., invalid entry type or missing title).
    #[error("validation error: {0}")]
    Validation(String),

    /// Indicates that a requested memory entry was not found.
    #[error("entry not found: {0}")]
    NotFound(String),

    /// Indicates a TARS role violation (e.g., a Builder attempting to write a Decision).
    #[error("role violation: {role} cannot write {entry_type}")]
    RoleViolation {
        /// The role reported by the agent.
        role: String,
        /// The type of entry they attempted to write.
        entry_type: String,
    },

    /// Wraps JSON serialization/deserialization errors.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Indicates that the project root could not be discovered.
    #[error("project root not found: {0}")]
    ProjectRoot(String),

    /// Represents internal server or logic errors.
    #[error("internal error: {0}")]
    Internal(String),

    /// Indicates a thread panic or lock poisoning.
    #[error("concurrency error: {0}")]
    Poison(String),
}
