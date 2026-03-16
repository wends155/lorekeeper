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
    /// A lesson learned from an event or bug.\
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
    /// A summary of an agent session — memory about memory.
    SessionSummary,
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
            Self::Deferred | Self::TechDebt | Self::SessionSummary => &["architect", "builder"],
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

/// Metadata for a `SessionSummary` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryData {
    /// ISO 8601 date of the session.
    pub session_date: String,
    /// Number of entries created during the session.
    pub entries_created: Option<u32>,
    /// Number of entries updated during the session.
    pub entries_updated: Option<u32>,
    /// Number of entries deleted during the session.
    pub entries_deleted: Option<u32>,
}

// ---------------------------------------------------------------------------
// Reflect types
// ---------------------------------------------------------------------------

/// Focus criteria for the `lorekeeper_reflect` tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReflectFocus {
    /// Entries not updated beyond the stale threshold.
    Stale,
    /// Entries never accessed since creation.
    Dead,
    /// Frequently accessed but potentially outdated entries.
    Hot,
    /// Entries with broken or stale `related_entries` links.
    Orphaned,
    /// Same-type entries with textual or tag overlap.
    Contradictions,
    /// Run all checks.
    #[default]
    All,
}

/// Overall maturity state of the memory bank.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    /// Zero entries exist.
    #[default]
    Empty,
    /// Fewer than 5 entries — insufficient data for full analysis.
    Nascent,
    /// 5–99 entries — full analysis available.
    Active,
    /// 100+ entries — full analysis plus density warnings.
    Mature,
}

/// Input parameters for `lorekeeper_reflect`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectCriteria {
    /// Which category of findings to surface.
    #[serde(default)]
    pub focus: ReflectFocus,
    /// Override the configured `stale_days` threshold for this call.
    pub stale_days: Option<u32>,
    /// Override the configured `hot_access_threshold` for this call.
    pub min_access_count: Option<u32>,
    /// Maximum number of findings to return (default: 20).
    pub limit: Option<u32>,
}

/// A single actionable finding from the reflect analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectFinding {
    /// Finding category (stale / dead / hot / orphaned / contradictions).
    pub category: String,
    /// UUID of the affected entry.
    pub entry_id: String,
    /// `EntryType` of the affected entry (serialized as string).
    pub entry_type: String,
    /// Title of the affected entry.
    pub title: String,
    /// Human-readable explanation of why this entry was flagged.
    pub reason: String,
}

/// Aggregate counts across all finding categories.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectSummary {
    /// Total number of findings.
    pub total: usize,
    /// Stale entry count.
    pub stale: usize,
    /// Dead entry count.
    pub dead: usize,
    /// Hot entry count.
    pub hot: usize,
    /// Orphaned entry count.
    pub orphaned: usize,
    /// Potential contradiction count.
    pub contradictions: usize,
}

/// Complete output of `lorekeeper_reflect`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReflectReport {
    /// Overall maturity classification of the memory bank.
    pub state: MemoryState,
    /// All findings up to the requested limit.
    pub findings: Vec<ReflectFinding>,
    /// Aggregate counts by category.
    pub summary: ReflectSummary,
    /// Optional guidance message (present when state = Empty or Nascent).
    pub guidance: Option<String>,
}

/// A candidate duplicate entry returned alongside a successful store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarEntry {
    /// UUID of the similar entry.
    pub id: String,
    /// Title of the similar entry.
    pub title: String,
    /// Entry type (as string).
    pub entry_type: String,
    /// BM25 similarity score (higher magnitude = more similar).
    pub score: f64,
}
