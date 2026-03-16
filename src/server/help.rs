//! Contextual help text for the `lorekeeper_help` MCP tool.
//!
//! Provides topic-based help strings and a dispatcher that maps topic names
//! to the appropriate human-readable guidance text.

const HELP_OVERVIEW: &str = "# Lorekeeper - Workflow Guide\n\n\
    Lorekeeper is your persistent structured memory bank. It survives across sessions.\n\n\
    ## Session Start\n\
    1. Call lorekeeper_stats - see current state of the memory bank\n\
    2. Call lorekeeper_recent - load recent context (last 10 entries)\n\
    3. Call lorekeeper_search - check for prior decisions and constraints\n\n\
    ## During Work\n\
    - Decision made -> lorekeeper_store type DECISION (architect role)\n\
    - Commit complete -> lorekeeper_store type COMMIT (builder role)\n\
    - Constraint found -> lorekeeper_store type CONSTRAINT (architect role)\n\
    - Lesson learned -> lorekeeper_store type LESSON (architect role)\n\
    - Plan created -> lorekeeper_store type PLAN (architect role)\n\
    - Work deferred -> lorekeeper_store type DEFERRED (either role)\n\
    - Tech debt noted -> lorekeeper_store type TECH_DEBT (either role)\n\n\
    ## Session End\n\
    - Update PLAN status to executed or abandoned via lorekeeper_update\n\
    - Resolve completed STUBs via lorekeeper_update\n\
    - Optionally call lorekeeper_render for a human-readable dump";

const HELP_ROLES: &str = "# Role Enforcement\n\n\
    | Role      | Allowed Types                                      |\n\
    |-----------|----------------------------------------------------||\n\
    | architect | DECISION, CONSTRAINT, LESSON, PLAN, FEATURE        |\n\
    | builder   | COMMIT, STUB, BUILDER_NOTE                         |\n\
    | both      | DEFERRED, TECH_DEBT                                |\n\n\
    Role violations are rejected server-side with a validation error.";

const HELP_TOOLS: &str = "# Lorekeeper Tools\n\n\
    ## Write\n\
    - lorekeeper_store  - create new entry\n\
    - lorekeeper_update - update existing entry\n\
    - lorekeeper_delete - soft-delete entry\n\
    - lorekeeper_render - export all entries as Markdown\n\n\
    ## Read\n\
    - lorekeeper_search  - full-text search (FTS5)\n\
    - lorekeeper_get     - retrieve by UUID\n\
    - lorekeeper_recent  - list newest entries\n\
    - lorekeeper_by_type - list filtered by type\n\
    - lorekeeper_stats   - aggregate counts\n\n\
    ## Help\n\
    - lorekeeper_help - this tool";

const HELP_DECISION: &str = "# DECISION\n\
    Records an architectural or technical decision.\n\
    - Role: architect only\n\
    - data: not required\n\
    - Use: when committing to an approach, library, or design pattern";

const HELP_COMMIT: &str = "# COMMIT\n\
    Records a git commit with hash and changed files.\n\
    - Role: builder only\n\
    - data: { hash: <sha>, files: [src/main.rs] }\n\
    - Use: after every significant git commit during Act phase";

const HELP_CONSTRAINT: &str = "# CONSTRAINT\n\
    Records a hard limit or restriction the project must respect.\n\
    - Role: architect only\n\
    - data: { source: <origin e.g. legal, infra, user> }\n\
    - Use: when discovering a constraint that affects design decisions";

const HELP_LESSON: &str = "# LESSON\n\
    Records a lesson learned from a bug, incident, or failed approach.\n\
    - Role: architect only\n\
    - data: { root_cause: <explanation> }\n\
    - Use: after resolving a non-trivial issue";

const HELP_PLAN: &str = "# PLAN\n\
    Records an implementation or migration plan.\n\
    - Role: architect only\n\
    - data: { scope: <area>, tier: S|M|L, status: planned }\n\
    - Status transitions: planned -> executed | planned -> abandoned\n\
    - Use: at the start of an Act phase; update to executed when complete";

const HELP_FEATURE: &str = "# FEATURE\n\
    Records a new feature or capability being designed.\n\
    - Role: architect only\n\
    - data: { status: <proposed|in-progress|done> }\n\
    - Use: when defining a new capability during Think phase";

const HELP_STUB: &str = "# STUB\n\
    Registers a placeholder for incomplete functionality across phases.\n\
    - Role: builder only\n\
    - data: { phase_number: N, contract: <desc>, module: <mod>, status: open }\n\
    - Status transitions: open -> resolved\n\
    - Use: when leaving a stub; close with lorekeeper_update when implemented";

const HELP_DEFERRED: &str = "# DEFERRED\n\
    Records work explicitly deferred to a future phase.\n\
    - Role: architect or builder\n\
    - data: { reason: <why>, target_phase: N }\n\
    - Use: when agreeing to defer a feature or fix";

const HELP_BUILDER_NOTE: &str = "# BUILDER_NOTE\n\
    Records an observation or suggestion during Act phase for Architect review.\n\
    - Role: builder only\n\
    - data: { note_type: tip|warn, step_ref: <step N>, plan_ref: <plan-id> }\n\
    - Use: when you notice something worth flagging but outside current scope";

const HELP_TECH_DEBT: &str = "# TECH_DEBT\n\
    Records a known technical debt item for future remediation.\n\
    - Role: architect or builder\n\
    - data: { severity: low|medium|high, origin_phase: N }\n\
    - Use: when introducing a deliberate shortcut or leaving something suboptimal";

const HELP_UNKNOWN: &str = "Unknown topic. Valid topics: overview, workflow, roles, tools, \
    DECISION, COMMIT, CONSTRAINT, LESSON, PLAN, FEATURE, STUB, DEFERRED, BUILDER_NOTE, TECH_DEBT";

/// Returns contextual help text for the given topic keyword.
///
/// Used by the `lorekeeper_help` MCP tool to dispatch topic strings to
/// human-readable guidance for LLM agents.
pub(super) fn get_help(topic: &str) -> &'static str {
    match topic {
        "overview" | "workflow" => HELP_OVERVIEW,
        "roles" => HELP_ROLES,
        "tools" => HELP_TOOLS,
        "DECISION" => HELP_DECISION,
        "COMMIT" => HELP_COMMIT,
        "CONSTRAINT" => HELP_CONSTRAINT,
        "LESSON" => HELP_LESSON,
        "PLAN" => HELP_PLAN,
        "FEATURE" => HELP_FEATURE,
        "STUB" => HELP_STUB,
        "DEFERRED" => HELP_DEFERRED,
        "BUILDER_NOTE" => HELP_BUILDER_NOTE,
        "TECH_DEBT" => HELP_TECH_DEBT,
        _ => HELP_UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_help_returns_known_topics() {
        assert!(get_help("overview").contains("Workflow Guide"));
        assert!(get_help("workflow").contains("Workflow Guide"));
        assert!(get_help("roles").contains("Role Enforcement"));
        assert!(get_help("tools").contains("lorekeeper_store"));
        assert!(get_help("PLAN").contains("planned"));
        assert!(get_help("STUB").contains("open"));
    }

    #[test]
    fn get_help_returns_fallback_for_unknown() {
        let result = get_help("nonexistent_topic");
        assert!(result.contains("Unknown topic"));
        assert!(result.contains("Valid topics"));
    }

    #[test]
    fn get_help_covers_all_individual_topics() {
        let topics = vec![
            "DECISION",
            "COMMIT",
            "CONSTRAINT",
            "LESSON",
            "FEATURE",
            "DEFERRED",
            "BUILDER_NOTE",
            "TECH_DEBT",
        ];
        for topic in topics {
            let result = get_help(topic);
            assert!(result.contains(topic), "Topic {topic} missing its header");
        }
    }
}
