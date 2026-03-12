use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntryType {
    Decision,
    Commit,
    Constraint,
    Lesson,
    Plan,
    Feature,
    Stub,
    Deferred,
    BuilderNote,
    TechDebt,
}

impl EntryType {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanData {
    pub scope: String,
    pub tier: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    pub hash: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintData {
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonData {
    pub root_cause: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureData {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubData {
    pub phase_number: u32,
    pub contract: String,
    pub module: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredData {
    pub reason: String,
    pub target_phase: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderNoteData {
    pub note_type: String,
    pub step_ref: String,
    pub plan_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDebtData {
    pub severity: String,
    pub origin_phase: u32,
}
