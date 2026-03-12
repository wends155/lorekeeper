---
description: How to perform a structured post-implementation audit (Reflect Phase)
---

# Audit Workflow

This workflow defines the standard process for auditing code against project standards.
It enforces the **Reflect** phase of the TARS protocol and generates a structured
**Audit Report** that feeds into `/plan-making` when findings require remediation.

> [!IMPORTANT]
> This workflow is **investigation-only** — no code edits, no fixes.
> Output is an **Audit Report** artifact. When findings exist, it feeds
> into `/plan-making` for remediation.

## Trigger

- `/audit` — Post-implementation audit (scoped to recently changed files).
- `/audit compliance` — On-demand compliance check (full codebase).

## Prerequisites

// turbo
> [!TIP]
> Run these auto-runnable commands to gather context and mechanical checks:
// turbo
> - `git diff --name-only HEAD~1` (changed files)
// turbo
> - Formatter check: `cargo fmt --all -- --check` *(or the command from `architecture.md § Toolchain`)*
// turbo
> - Linter: `cargo clippy --all-targets --all-features -- -D warnings`
// turbo
> - Tests: `cargo test --all-features`
// turbo
> - Unwrap scan: `rg -n "\.unwrap\(\)" src/ --glob "*.rs" --glob "!*test*"`
// turbo
> - Secret scan: `rg -n -i "API_KEY\s*=|SECRET\s*=|PASSWORD\s*=|TOKEN\s*=" . --glob "!.git" --glob "!target" --glob "!*.lock"`
// turbo
> - TODO markers: `rg -n "TODO|FIXME|HACK" --type-add "code:*.{rs,go,ts,js,svelte}" --type code .`
>
> If Narsil MCP is available, also run `scan_security` and `check_cwe_top25`. Use `-Scope full` for compliance mode.

- Read `.agent/rules/audit-rules.md` for report format, finding classification, and verdict criteria.
- Read `architecture.md` (if present) for project-specific design and toolchain.
- Read `.agent/rules/coding-standard.md` (if present) for language-specific coding standards.
- Read `context.md` (if present) for historical decisions.
- If post-implementation: the original implementation plan is available for cross-reference.
- Confirm you are operating as the **Architect** (high-reasoning model).

## Steps

### 1. Gather Context

Before auditing, collect all relevant materials:

- **Scope**: Determine if this is a post-implementation audit (changed files) or a full compliance check.
- **Changed Files**: `git diff --name-only` to identify what was created, modified, or deleted.
- **Implementation Plan**: Locate and re-read the original approved plan (post-implementation only).
- **Verification Logs**: Review any test output, lint results, or build logs from the Act phase.
- **Git Diff**: Run `git diff` or `git log` to see the exact changes made.

### 2. Compliance Audit

Systematically verify the code against project standards.

#### 2a. Plan Fidelity *(post-implementation only; see GEMINI.md §7)*
- [ ] Every plan item maps to a `[x]` in `task.md` and a corresponding `git diff`
- [ ] No unapproved changes were introduced (check for Additions per Fidelity Matrix)
- [ ] If deviations occurred, they are documented with justification
- [ ] Builder Notes section of `task.md` reviewed and processed (see §2a-bis)
- [ ] No stale stubs remain: `STUB(Phase N)` where N ≤ current phase are all addressed *(multi-phase only — verify with `rg "STUB\(Phase"`)*

#### 2a-bis. Builder Notes Processing *(if Builder Notes exist in task.md)*

Review each note in the `## Builder Notes` section:

- **💡 Suggestions**: Promote to a future plan backlog item, or dismiss with brief rationale.
- **⚠️ Observations**: Acknowledge and record in `context.md` if relevant to future work.

> [!NOTE]
> Builder Notes are informational — they were logged during `/build` per
> `builder-rules.md §7`. The Architect decides what action (if any) to take.

#### 2b. GEMINI.md Compliance *(skip items already covered by §2f)*
- [ ] **Error Handling**: No silent failures; errors communicate what/where/why
- [ ] **Observability**: Structured logging present for significant operations
- [ ] **Documentation**: All public functions/modules have doc comments

#### 2c. Testing & Testability *(skip items already covered by §2f)*
- [ ] **Unit/integration tests** exist for all new/changed logic
- [ ] **Edge cases**: Tests cover boundary conditions, empty inputs, error paths, and fringe scenarios
- [ ] **Mocks & stubs**: External dependencies are abstracted behind interfaces/traits and mocked in tests
- [ ] **Testable design**: Code avoids tight coupling to global state, filesystems, or network — dependencies are injectable
- [ ] **No crashes**: No unhandled exceptions, raw panics, or uncontrolled termination paths remain untested

#### 2d. Architecture Compliance *(if `architecture.md` exists)*
- [ ] Code follows the project's directory structure and layout conventions
- [ ] Error handling uses the project's designated strategy
- [ ] Logging uses the project's designated framework
- [ ] Testing follows the project's designated framework and conventions
- [ ] Dependencies are declared correctly
- [ ] Any new patterns are consistent with existing architecture

#### 2e. Code Quality
- [ ] Code is idiomatic for the language
- [ ] No dead code, unused imports, or commented-out blocks
- [ ] No hardcoded secrets, credentials, or environment-specific values
- [ ] Variable/function names are clear and descriptive
- [ ] Complex logic has explanatory comments

#### 2f. Coding Standards Compliance *(if `coding-standard.md` exists)*
- [ ] For each applicable section in `coding-standard.md`, verify the changed code complies
- [ ] Focus on sections relevant to the patterns used (error handling, async, testing, etc.)
- [ ] No prohibited patterns as listed in the coding standard's quick reference
- [ ] Linter/formatter config matches the coding standard's toolchain section

#### MCP-Enhanced Audit *(when available)*

If **Narsil MCP** is available, use it to automate specific checklist items:

| Checklist Item | Narsil Tool | Section |
|---|---|---|
| Dead code | `find_dead_code` | 2e |
| Unused exports | `find_unused_exports` | 2e |
| Hardcoded secrets | `scan_security` (secrets ruleset) | 2e |
| Error handling (CWE) | `check_cwe_top25` | 2b, 2f |
| Security vulnerabilities | `check_owasp_top10` | 2e |
| Prohibited patterns | `find_similar_code` against anti-patterns | 2f |
| Dependency structure | `get_import_graph`, `find_circular_imports` | 2d |
| Type errors | `check_type_errors` | 2e |
| `.unwrap()` usage | `search_code` excluding test files | 2f |

For **multi-file audits** (>5 changed files), the Architect **SHOULD** use `sequentialthinking` to:
- Structure the audit across many files systematically.
- Reason through complex compliance violations with multiple contributing factors.
- Prioritize findings by severity and impact.

For **single-file audits**, skip sequential thinking — the overhead isn't worth it.

### 3. Verification Gate

Re-run the project's standard verification pipeline and confirm zero-exit:

| Check | Command | Status |
|-------|---------|--------|
| **Formatter** | *Refer to `architecture.md` § Toolchain* | ☐ Pass |
| **Linter** | *Refer to `architecture.md` § Toolchain* | ☐ Pass |
| **Tests** | *Refer to `architecture.md` § Toolchain* | ☐ Pass |

> [!IMPORTANT]
> Do NOT invent commands. Source them from `architecture.md` § Toolchain.
> If `architecture.md` is absent, inspect build/config files to determine correct commands.
> For Rust projects, also verify that clippy lint levels match `coding-standard.md` § 3.2.

// turbo
> [!TIP]
> Run the full verification pipeline (commands from `architecture.md § Toolchain`):
// turbo
> - `cargo fmt --all -- --check`
// turbo
> - `cargo clippy --all-targets --all-features -- -D warnings`
// turbo
> - `cargo test --all-features`
>
> All three must exit 0.

### 4. Audit Report

Document the audit results following the format in `audit-rules.md` §1.
Classify each finding per `audit-rules.md` §2 (categories and severity).

> [!CAUTION]
> Do **not** include proposed solutions, fixes, or implementation suggestions.
> The Audit Report is a diagnostic input for `/plan-making`, not a plan.

### 5. Verdict & Handoff

Determine the verdict per `audit-rules.md` §3. For post-implementation audits, also apply the Fidelity Matrix per `audit-rules.md` §4.

Present the verdict and handoff options to the user:

- **✅ Pass**: Proceed to Summarize (Step 6).
- **⚠️ Pass with notes**: "Reply with **Plan** to remediate, or **Accept** to proceed."
- **📖 Documentation-only findings**: "Reply with **Docs** for `/update-doc`, or **Plan** for `/plan-making`."
- **❌ Fail**: "Reply with **Plan** to create a remediation plan."

> [!NOTE]
> If this is the **second consecutive audit failure** for the same scope,
> escalate to the user rather than re-entering the plan→build→audit cycle.

**Do NOT tell the Builder to fix without a plan.** All remediations must go through
`/plan-making` to enforce the TARS Planning Gate.

### 6. Summarize (Context Compression)

**Critical:** This step prevents context bloat per TARS protocol rules.

After a passing audit (or accepted pass-with-notes), compress the interaction:

> 📝 **Context Update:**
> * **Feature:** [Name of the feature/change]
> * **Changes:** [Summary of logic/files changed]
> * **New Constraints:** [Any new rules for future Think phases]
> * **Pruned:** [What technical debt/logs can now be ignored]

- If `context.md` exists, append this update to it.
- If `context.md` does not exist, create it with this as the first entry.

### 7. Completion

End the audit with:

> ✅ **Reflect Phase Complete.** Context has been compressed.

The task is now considered fully closed under the TARS protocol.

## Rules

1. **Always pause** — the user must approve findings before proceeding.
2. **Classify findings** — every finding must have Category, Severity, File, and Rule.
3. **Use MCP tools** — prefer Narsil and Sequential Thinking when available for accuracy.
4. **Preserve passing items** — document compliant items too, not just failures.
5. **Respect the Planning Gate** — never tell the Builder to fix without routing through `/plan-making`.
6. **Auto-run** — see `GEMINI.md` §6 Auto-Run table. Judge by base command. **Avoid `2>&1`** — the IDE blocks auto-run on any `>` syntax.
