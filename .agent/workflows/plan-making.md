---
description: How to create a high-quality implementation plan (Think Phase)
---

# Plan-Making Workflow

This workflow defines the standard process for creating implementation plans.
It enforces the Planning Gate and Think Phase of the TARS protocol.

## Prerequisites

> [!TIP]
> Load context using native agent tools (zero-prompt):
> 1. Read `architecture.md`, `context.md`, `.agent/rules/coding-standard.md`, and `.agent/rules/ipr.md` with `view_file` (if they exist).
> 2. Run these auto-runnable commands:
// turbo
>    - `git log -n 20 --oneline`
// turbo
>    - `rg -n "TODO|FIXME|HACK" --type-add "code:*.{rs,go,ts,js,svelte,py}" --type code .`

- Read `architecture.md` (if present) for project-specific design, toolchain, and patterns.
- Read `.agent/rules/coding-standard.md` (if present) for language-specific coding standards.
- Read `.agent/rules/ipr.md` (if present) for implementation plan format and handoff rules.
- Read `context.md` (if present) for historical decisions and prior context.
- If the plan scope requires multiple phases (assessed during Step 1), also read `.agent/rules/phase-rules.md` for phase manifest format, STUB conventions, and phase gate requirements.
- If a Report was produced by `/issue`, `/audit`, or `/feature`, use it as the **primary input** for Step 1. Do not re-investigate areas already covered.
- Confirm you are operating in **Planning mode** (no code edits allowed).

## Steps

### 1. Scope & Impact Analysis

Investigate the request before writing anything:

- **Identify affected files**: List every file/module that will be touched.
- **Map dependencies**: What depends on those files? What do they depend on?
- **Flag risks**: Security concerns, breaking changes, performance impacts.
- **Check for existing tests**: Search for test files related to the affected code.
- **Assess phase scope**: Determine if this is a single-phase or multi-phase plan. Multi-phase indicators: scope touches >3 modules, requires infrastructure setup before features, or depends on stubs from prior phases. If multi-phase, load `phase-rules.md` and include Phase Context / Phase Manifest in the plan.

#### MCP-Enhanced Analysis *(when available)*

If **Narsil MCP** is available, use it throughout planning:

**Investigation** (Step 1):

| Tool | Purpose |
|------|---------|
| `get_import_graph`, `get_dependencies` | Visualize what's affected by proposed changes |
| `find_circular_imports`, `check_cwe_top25`, `check_owasp_top10` | Catch structural or security risks early |
| `find_symbols`, `find_references`, `get_symbol_definition` | Understand interfaces before proposing changes |
| `find_unused_exports`, `find_dead_code` | Identify cleanup opportunities to include in the plan |

**Validation** (Step 2 — use results to support proposed changes):

| Tool | Purpose |
|------|---------|
| `get_symbol_definition` | Verify interfaces/types being modified exist as expected |
| `find_references` | Check blast radius of proposed signature changes |
| `check_dependencies` | Check for vulnerable deps before adding new ones |
| `find_similar_code` | Find existing patterns the plan should follow |

For **M/L tier plans**, the Architect **SHOULD** use `sequentialthinking` to break down complex changes, reason about ordering, and validate root cause coverage before drafting. For **S-tier plans**, skip it — the overhead isn't worth it.

### 2. Draft the Plan

Follow the plan format, revision protocol, and handoff rules defined in `.agent/rules/ipr.md`.

### 3. Sync task.md (Agent Procedure)

Generate `task.md` from the plan:

1. Read the plan file with `view_file`.
2. Extract all headings matching `### ComponentName` and `#### [NEW|MODIFY|DELETE|TEST] filename`.
3. Write `task.md` to the same directory as the plan:

   ```markdown
   # Task: <plan title from first # heading>

   ## Objectives
   - [ ] <Component 1>
     - [ ] [ACTION] <filename>
   - [ ] <Component 2>
     - [ ] [ACTION] <filename>
   - [ ] Run verification pipeline
   - [ ] Update docs
   - [ ] Update context.md
   - [ ] Commit
   ```

> [!WARNING]
> task.md must be aligned with the plan before requesting approval.

> [!IMPORTANT]
> **Test Granularity Rule (RC-2 Prevention):** Each individually named test function in the plan's
> Test Plan table must appear as its own `- [ ]` sub-item within the corresponding `[TEST]` entry
> in task.md. A single `[TEST] filename — unit tests` checkbox is **not** sufficient if the Test
> Plan enumerates multiple named tests. The Builder checks off each test individually.

### 4. Self-Review Checklist

Before requesting approval, verify each item. Items marked 🤖 can be verified
with Narsil MCP or scripts; items marked 🧠 require LLM judgment.

**Scope & Coverage:**
- [ ] 🤖 All affected files are listed (verify with Narsil `find_references`)
- [ ] 🤖 Each change is broken into numbered, independently verifiable steps
- [ ] 🧠 Module boundaries defined (Owns / Does NOT own)
- [ ] 🧠 Interface contracts specified (signatures, invariants, error conditions)
- [ ] 🧠 Cross-module handshakes documented (caller/callee, data format, error propagation)
- [ ] 🧠 Code snippets included for non-trivial changes
- [ ] 🧠 Steps implementing non-trivial logic (>1 branch or algorithm) include inline pseudocode **or** an explicit `See architecture.md §X` cross-reference — vague descriptions like "walk-up discovery" without the algorithm are not acceptable (RC-3 Prevention)
- [ ] 🧠 If `coding-standard.md §4.5` requires doc comments on public items, plan includes doc-writing steps **or** explicitly lists docs as deferred in the Out-of-Scope section (RC-5 Prevention)

**Compliance** (cross-reference each proposed change against these rules):

| Rule Source | Check |
|-------------|-------|
| GEMINI.md § Error Handling | New functions handle errors with what/where/why; no silent failures |
| GEMINI.md § Observability | Plan includes structured logging for significant operations |
| GEMINI.md § Testing | Test Plan covers all new/changed logic |
| GEMINI.md § Documentation | New public APIs will have doc comments |
| coding-standard.md *(if present)* | Error handling (§4.1), async (§4.2), patterns (§4.6), module org (§4.7), observability (§4.8), defensive programming (§4.9), prohibited patterns (§10) |
| architecture.md *(if present)* | Layout conventions, toolchain commands |

> [!CAUTION]
> If any proposed change cannot satisfy a rule, document the exception with
> justification in the Edge Cases & Risks section. Do not silently skip compliance.

**Process:**
- [ ] 🤖 No code was edited (Planning Gate enforced)
- [ ] 🧠 `context.md` consulted for historical decisions (if present)
- [ ] 🧠 Constraints clearly documented in Problem Statement
- [ ] 🤖 Dependencies researched — check with Narsil `check_dependencies`
- [ ] 🧠 Risks and edge cases documented
- [ ] 🧠 Mermaid diagram included for structural changes

**Integration:**
- [ ] 🤖 Report findings incorporated (if `/issue`, `/audit`, or `/feature` was run)
- [ ] 🤖 MCP tools used for investigation/analysis where available
// turbo
- [ ] 🤖 task.md synced — run the **Validate task.md** procedure:
  1. Read both `task.md` and the plan file with `view_file`.
  2. Check that every `[NEW|MODIFY|DELETE|TEST] filename` in the plan appears in `task.md`.
  3. Check that every such entry in `task.md` appears in the plan.
  4. If mismatches → report and STOP.

### 5. Pre-flight Gate (Agent Procedure)

Verify before requesting approval:

1. `task.md` exists — check with `view_file`; if not found, STOP.
2. `task.md` contains `[ ]`, `[x]`, or `[/]` checklist markers.
3. Run the **Validate task.md** procedure above — must pass.
4. If all three pass → proceed to request approval.

> [!CAUTION]
> The pre-flight gate MUST pass before requesting approval.
> If it fails, fix the issues and re-check. Do NOT skip this step.

End the plan with:

> 🛑 **Think Phase Complete.** Reply with **"Proceed"** to Act.

Do NOT proceed to implementation until the user explicitly approves.

### 6. Post-Approval Handoff

Once approved, follow **GEMINI.md §6 Handoff Protocol** for the full Act cycle.
After `/audit` passes, run `/update-doc` scoped to affected files, then summarize in `context.md` per GEMINI.md §8.
