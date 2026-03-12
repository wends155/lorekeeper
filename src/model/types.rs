//! Entry type definitions and associated metadata schemas.

use serde::{Deserialize, Serialize};

/// The category of a memory entry, determining its role permissions and schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryType {
    /// A high-level architectural decision.
    Decision,
    /// A code commit reference with hash and files.
    Commit,
    /// A technical or project constraint.
    Constraint,
    /// A lesson learned from an event or bug.
    Lesson,
    /// An implementation or migration plan.
    Plan,
    /// A new feature or capability description.
    Feature,
    /// A placeholder for incomplete functionality.
    Stub,
    /// A task or feature deferred to a future phase.
    Deferred,
    /// Internal notes for the Builder role.
    BuilderNote,
    /// Technical debt markers.
    TechDebt,
}

impl rusqlite::ToSql for EntryType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        // Remove quotes from JSON string
        let s = s.trim_matches('"').to_owned();
        Ok(rusqlite::types::ToSqlOutput::from(s))
    }
}

impl rusqlite::types::FromSql for EntryType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        serde_json::from_str(&format!("\"{s}\""))
            .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
    }
}

impl EntryType {
    /// Returns the roles authorized to create/write this entry type.
    #[must_use]
    pub const fn allowed_roles(self) -> &'static [&'static str] {
        match self {
            Self::Decision | Self::Constraint | Self::Lesson | Self::Plan | Self::Feature => {
                &["architect"]
            }
            Self::Commit | Self::Stub | Self::BuilderNote => &["builder"],
            Self::Deferred | Self::TechDebt => &["architect", "builder"],
        }
    }
}

// Type-specific data structs

/// Metadata for a `Plan` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanData {
    /// The scope of the plan (e.g., "module", "project").
    pub scope: String,
    /// The TARS tier (S, M, L).
    pub tier: String,
    /// Execution status (e.g., "draft", "approved").
    pub status: String,
}

/// Metadata for a `Commit` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    /// Full or short git commit hash.
    pub hash: String,
    /// List of file paths modified in this commit.
    pub files: Vec<String>,
}

/// Metadata for a `Constraint` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintData {
    /// Source of the constraint (e.g., "legal", "infrastructure").
    pub source: String,
}

/// Metadata for a `Lesson` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonData {
    /// Explanation of why the lesson was necessary.
    pub root_cause: String,
}

/// Metadata for a `Feature` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureData {
    /// Current development status of the feature.
    pub status: String,
}

/// Metadata for a `Stub` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubData {
    /// The project phase this stub belongs to.
    pub phase_number: u32,
    /// Behavioral contract description.
    pub contract: String,
    /// Target module for implementation.
    pub module: String,
    /// Implementation status.
    pub status: String,
}

/// Metadata for a `Deferred` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredData {
    /// Reason for deferring.
    pub reason: String,
    /// Intended phase for implementation.
    pub target_phase: u32,
}

/// Metadata for a `BuilderNote` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderNoteData {
    /// Classification of the note (e.g., "observation").
    pub note_type: String,
    /// Reference to a plan step number.
    pub step_ref: String,
    /// Reference to the implementation plan ID.
    pub plan_ref: String,
}

/// Metadata for a `TechDebt` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDebtData {
    /// Impact level (e.g., "low", "high").
    pub severity: String,
    /// Phase where the debt was introduced.
    pub origin_phase: u32,
}
