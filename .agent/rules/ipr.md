# Implementation Plan Rules (IPR)

> Loaded by `/plan-making` workflow. Defines plan format, revision protocol, and handoff rules.

## 1. The Planning Gate

- **Trigger:** "Plan", "Draft", "Propose", "Design" → Agent is Language-Locked (Markdown only).
- **Prohibited:** Editing source code or core documentation (except to read).
- **Allowed:** Creating/Editing implementation plans in an artifacts directory.
- **Exit:** Agent **MUST** pause and request user approval. Unlock only after "Proceed"/"Approved".
- **Output:** Always an artifact, never code changes.

## 2. Plan Format

### Scaling Tiers

| Tier | Files | Required Sections |
|------|-------|-------------------|
| **S** (patch) | 1-3 | Header, Problem Statement, Global Execution Order, Verification Plan, Plan Summary |
| **M** (feature) | 4-10 | + Builder Context, Interface Contracts, Test Plan, Negative Scope, Phase Context *(if multi-phase)* |
| **L** (refactor) | 10+ | + Module Boundaries, Cross-Module Handshakes, Architecture Diagram, Dependency Chain, Phase Manifest *(if multi-phase)* |

### Header

| Field | Value |
|-------|-------|
| **Role** | Architect / Builder |
| **Date** | Current date |
| **Scope** | One-line summary of what changes |
| **Tier** | S / M / L |

> All code produced must comply with `.agent/rules/coding-standard.md`.

### Builder Context *(M/L tier)*

List exact files and line ranges the Builder must read before starting:

```markdown
## Builder Context
Read before starting:
- `src/lib.rs` L1-30 (module structure)
- `src/error.rs` (current error types)
- `architecture.md § Error Handling` (project convention)
```

### Phase Context *(M/L tier, multi-phase only)*

For plans that are part of a multi-phase project (see `phase-rules.md`):

```markdown
## Phase Context
- **Phase:** 2 of 5 (Core Feature)
- **Prior phase:** Phase 1 delivered foundation (config, errors, DB, tracing)
- **Stubs for this phase:** STUB(Phase 2) items from prior Phase Manifest:
  - `MockPaymentGateway` in src/infra/payment.rs → replace with Stripe integration
  - `NoOpNotifier` in src/infra/notify.rs → replace with webhook dispatcher
- **Reference:** See `phase-rules.md` for full conventions
```

> [!NOTE]
> Phase Context is only required for multi-phase plans. Single-phase plans omit this section.
> The Architect determines if a plan is multi-phase during scope analysis in `/plan-making`.

### Problem Statement

- What is the problem or feature request?
- Why does it need to be solved now?
- Any relevant context from `context.md` or prior conversations.
- **Constraints**: Technical limitations, environment restrictions, performance budgets, or scope boundaries.
- **Dependencies**: What existing libraries, crates, or packages can be leveraged? Check the ecosystem before proposing custom solutions.

> [!NOTE]
> If an input report exists from `/issue`, `/audit`, or `/feature`,
> cross-reference each proposed change against the report's findings.
> Every finding should map to a proposed change.

### Negative Scope *(M/L tier)*

Explicitly list what the Builder must NOT touch:

```markdown
## Out of Scope
- Do NOT modify `src/config.rs` (separate plan)
- Do NOT add new dependencies
- Do NOT refactor existing tests
```

### Interface Contracts *(M/L tier)*

For every new or changed public function, struct, or trait:
- Exact signature (name, params, return type, error type).
- Invariants (preconditions, postconditions).
- Error conditions (what can fail and what the caller gets back).

### Module Boundaries *(L tier only)*

For each component group:
- **Owns**: What this module is responsible for.
- **Does NOT own**: What's delegated to other modules.

### Cross-Module Handshakes *(L tier only)*

When a change affects callers/callees across modules:
- Caller → Callee with the exact function/method.
- Data format exchanged (types, ownership).
- Error propagation path across the boundary.

### Global Execution Order *(all tiers)*

> [!IMPORTANT]
> Number ALL steps globally across ALL files. The Builder follows steps 1, 2, 3... linearly.
> No per-file numbering. No jumping.

Each step is verification-oriented:

```markdown
Step N: [NEW/MODIFY/DELETE/TEST] file_path — function_name() (L##-##)
- Pre: CHECK
- Target: function/struct name + line range
- Action: what to change (code snippet or description)
- Post: CHECK, no anyhow in file
- 🔒 CHECKPOINT (only on steps requiring ALL)
```

**Tags:**
- `[NEW]` — create a new file
- `[MODIFY]` — change existing code
- `[DELETE]` — remove a file or code block
- `[TEST]` — write or update a test (TDD Red step)

**Pre/Post Vocabulary:**

| Shorthand | Meaning | Default Command |
|-----------|---------|-----------------|
| `CHECK` | Type-check compiles | `cargo check` |
| `FMT` | Format passes | `cargo fmt --check` |
| `CLIPPY` | Lint passes | `cargo clippy -- -D warnings` |
| `TEST` | Tests pass | `cargo test` |
| `BUILD` | Full build | `cargo build` |
| `ALL` | FMT + CLIPPY + TEST | Full pipeline |

> [!NOTE]
> Default commands shown. Projects override in `architecture.md § Toolchain`.

Pre/Post can combine shorthand with conditions: `Post: CHECK, no anyhow imports in file`.

**🔒 CHECKPOINT** marks where the Builder runs `ALL` and commits.

**Checkpoint frequency:** At minimum, place `🔒` after each component group and after every `[TEST]` step. For S-tier plans, one `🔒` at the end suffices.

### Dependency Chain *(L tier)*

Show which steps depend on which:

```
1 → 2 → 3
         ↘
     4 → 5 → 6 🔒
```

### Architecture Diagram *(if applicable, M/L tier)*

Include a Mermaid diagram for any structural or data-flow changes.

### Edge Cases & Risks

List edge cases the implementation must handle. Document risks or trade-offs.

### Test Plan (TDD) *(M/L tier)*

> [!IMPORTANT]
> Plans **must** specify tests **before** implementation code. The Builder writes
> tests first, verifies they fail (Red), then implements until they pass (Green).

For each proposed change, define:

1. **Test cases**: Function signatures and assertions — written as executable code, not prose.
2. **Test type**: Unit, integration, property-based, or doc-test.
3. **Expected failures**: What the test asserts when run *before* implementation.
4. **Test file location**: Co-located `#[cfg(test)]` module or dedicated test file.

**Code snippets as executable tests:** Instead of describing expected output in prose,
express verification as a test assertion. The plan's code should be testable, not illustrative.

### Verification Plan *(all tiers)*

| Type | Required? | Details |
|------|-----------|---------|
| **Automated tests** | Yes | Exact command (e.g., `cargo test`, `npm test`) |
| **Lint / Format** | Yes | Exact command (e.g., `cargo fmt --check`) |
| **Manual testing** | If applicable | Step-by-step instructions |
| **Browser testing** | If applicable | Specific pages/flows |

> [!IMPORTANT]
> Do NOT invent test commands. Refer to `architecture.md § Toolchain`.

### Plan Summary *(all tiers)*

| Metric | Value |
|--------|-------|
| Tier | S / M / L |
| Files | N |
| Steps | N |
| Checkpoints | N |
| Estimated effort | Low / Medium / High |

## 3. Revision Protocol

When revising an approved or in-progress plan:

1. **Targeted edits only** — Use `replace_file_content` or `multi_replace_file_content`.
   Do NOT rewrite the entire plan. Unchanged sections must be preserved verbatim.
2. **Mark revisions** — Tag changed sections with `[REVISED]` so the diff is visible.
3. **No summarization** — Never condense unchanged content. If a section wasn't
   discussed in the revision, do not touch it.
4. **Re-sync task.md** — After any revision, re-run `pwsh .agent/scripts/Sync-TaskList.ps1 -Mode validate`.

## 4. Decision Resolution

When a plan presents options and the user decides:

1. **Delete rejected options entirely** — Do not leave them as "rejected" or "not chosen."
2. **State the chosen option as fact** — No "we chose X over Y"; just state X.
3. **Log the rationale** — Record what was decided and why in the plan's Problem Statement
   or in `context.md` so the reasoning isn't lost.
4. **Sweep for strays** — After resolving, use `rg` to catch lingering references:
   ```powershell
   rg "Option A|Option B|Alternative|vs\." <plan-file>
   ```

## 5. Handoff-Ready Requirements

Before the Architect can request "Proceed", the plan must satisfy:

| Requirement | Verification |
|-------------|-------------|
| Every file listed with `[NEW]`/`[MODIFY]`/`[DELETE]`/`[TEST]` tags | Manual review |
| Every change has a discrete, verifiable description | Manual review |
| Test cases pre-specified (TDD: Red → Green → Refactor) | Test Plan section exists |
| Verification commands sourced from `architecture.md § Toolchain` | Cross-reference check |
| Plan Summary filled in | Manual review |
| `task.md` aligned per §6 | `Sync-TaskList.ps1 -Mode validate` exits 0 |

## 6. The task.md Contract

`task.md` is the bridge between the Architect's plan and the Builder's execution:

1. **Generated** by `Sync-TaskList.ps1 -Mode generate` — writes `task.md` to the plan directory automatically.
2. **1:1 Mapping**: Each checklist item maps to exactly one plan item.
3. **Progress Tracking**: Builder marks `[ ]` → `[/]` (in-progress) → `[x]` (done).
4. **Validation Gate**: Before each commit, `Sync-TaskList.ps1 -Mode validate` must exit 0.
5. **Pre-flight**: Before plan approval, `Sync-TaskList.ps1 -Mode preflight` must exit 0.

## 7. Builder Obligations & STOP Conditions

**Obligations:**
1. Execute plan items in order — no deviations.
2. If a plan item is unclear or flawed → **STOP**, request re-audit.
3. Update `task.md` in the artifacts directory after each file modification:
   - Mark `[ ]` → `[/]` when starting a step.
   - Mark `[/]` → `[x]` when the step passes verification.
   - Antigravity reads this file for UI progress — stale markers hide progress from the user.
4. Run `ALL` at each 🔒 CHECKPOINT.
5. Use `Git-Checkpoint.ps1` for atomic commits tied to task.md progress.
6. 🛑 **Wait for user instruction** before pushing to remote repositories.

**STOP Conditions** — Builder must immediately halt and return to the Architect when:
- The plan contradicts `architecture.md`.
- A plan item is ambiguous or untestable.
- An unplanned dependency or breaking change is discovered.
- The second consecutive test failure occurs on the same item.

**On STOP:** Commit current progress with message `WIP: stopped at step N — [reason]`.
Do NOT revert completed steps. The Architect decides rollback scope during re-planning.
If a step broke prior work, note the regression in the STOP report.

## 8. Resumption Protocol

When resuming a plan in a new session:

1. Read `context.md` for prior session summary.
2. Read `task.md` — identify last `[x]` item and first `[ ]` item.
3. Verify partial state: run `ALL` to confirm prior work still passes.
4. Resume from the first `[ ]` step. Do not re-execute `[x]` steps.
5. If `ALL` fails on prior work → STOP, escalate (do not silently fix).

## 9. Example: S-Tier Plan

```markdown
**Role:** Architect · **Date:** 2026-02-26 · **Tier:** S

### Problem Statement
`parse_config()` panics on empty input. Should return `ConfigError::EmptyInput`.

### Global Execution Order

Step 1: [TEST] src/config.rs — test_empty_input()
- Pre: ALL
- Target: #[cfg(test)] mod tests
- Action: Add test asserting parse_config("") returns Err(ConfigError::EmptyInput)
- Post: TEST fails (Red — function doesn't handle empty input yet)

Step 2: [MODIFY] src/config.rs — parse_config() (L12-30)
- Pre: Step 1 test exists and fails
- Target: parse_config() L12
- Action: Add early return: if input.is_empty() { return Err(ConfigError::EmptyInput); }
- Post: ALL 🔒

### Verification Plan
| Type | Command |
|------|---------|
| Tests | cargo test |
| Lint | cargo clippy -- -D warnings |

### Plan Summary
| Metric | Value |
|--------|-------|
| Tier | S |
| Files | 1 |
| Steps | 2 |
| Checkpoints | 1 |
| Estimated effort | Low |
```
