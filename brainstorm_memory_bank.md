# Brainstorm: Agent Long-Term Memory Bank (Rust MCP Server)

**Date:** 2026-03-12  
**Status:** Brainstorm complete — ready for `/feature` or `/architecture` in target workspace

---

## Problem

`context.md` is a flat markdown file that stores all project history. At 36K+ tokens and growing, it consumes significant context window on every session while the agent uses ~5% of its content. No selective retrieval, no typing, no search.

## Solution

A **Rust MCP server** wrapping **SQLite + FTS5** that acts as structured long-term memory. The agent reads/writes typed entries via MCP tools instead of appending markdown. The human reviews via a rendered markdown export.

## Entry Types

| Type | Required Fields | Role Scope |
|------|----------------|------------|
| `DECISION` | title, body (must include "why"), tags | Architect only |
| `COMMIT` | hash (valid SHA), message, files | Builder only |
| `CONSTRAINT` | title, body, source (doc reference) | Architect only |
| `LESSON` | title, body, root_cause | Architect only |
| `PLAN` | title, scope, tier (S/M/L), status (PLANNED/EXECUTED/ABANDONED) | Architect only |
| `FEATURE` | title, status (EXPLORING/PLANNED/IMPLEMENTED/DEFERRED) | Architect only |
| `STUB` | phase_number, contract, module, status (OPEN/RESOLVED) | Builder writes, Architect reads |
| `DEFERRED` | description, reason, target_phase | Both |
| `BUILDER_NOTE` | type (💡/⚠️), step_ref, plan_ref, body | Builder only |
| `TECH_DEBT` | description, severity, origin_phase | Both |

All entries also have: `id`, `timestamp`, `tags[]`, `related_entries[]`, `role` (who wrote it).

## MCP Tools

| Tool | Purpose |
|------|---------|
| `memory_store(type, fields...)` | Insert entry. Server validates required fields + role permissions. Rejects bad data with structured error. |
| `memory_search(query, type?, limit?)` | FTS5 keyword search with optional type filter |
| `memory_recent(n)` | Last N entries across all types |
| `memory_by_type(type, filters?)` | All entries of a type, with optional status/plan filters |
| `memory_by_commit(hash)` | All entries related to a commit |
| `memory_stats()` | Entry counts by type, staleness, coverage gaps |
| `memory_render(format?)` | Export to markdown for human review |
| `memory_import(file, strategy)` | One-time migration from context.md |

## Workflow Integration

| TARS Phase | Reads | Writes |
|------------|-------|--------|
| Bootstrap (`/toolcheck`) | `memory_stats()` | — |
| Pre-Think (`/issue`, `/feature`) | `memory_search(topic)` | — |
| Think (`/plan-making`) | `memory_by_type(CONSTRAINT)` + `memory_search(modules)` | PLAN (status=PLANNED) |
| Act (`/build`) | `memory_by_commit(baseline)` + stubs/deferred for current phase | COMMIT, BUILDER_NOTE, STUB |
| Reflect (`/audit`) | `memory_search(plan)` + `memory_by_type(BUILDER_NOTE)` | LESSON |
| Summarize | Session review | DECISION, CONSTRAINT, update PLAN→EXECUTED |

## Role Enforcement

The MCP server enforces TARS role boundaries mechanically:
- Builder can't write DECISIONs or CONSTRAINTs
- Architect can't write COMMITs (signals Planning Gate violation)
- BUILDER_NOTEs are builder-only
- Role is passed with every `memory_store` call and validated server-side

## Multi-Phase Support

- `STUB` entries replace scanning source code for `// STUB(Phase N)` comments
- `DEFERRED` entries persist across sessions (unlike task.md which gets overwritten)
- Phase boundary queries: `memory_search(plan="Phase 1", type=[STUB, DEFERRED, TECH_DEBT])`

## Stack

- **Language:** Rust
- **Database:** SQLite via `rusqlite`
- **Search:** SQLite FTS5 (built-in, no external deps)
- **Protocol:** MCP (stdio transport, same as Narsil)
- **Vectors:** Not initially. FTS5 + typed entries cover 90%. Add later if needed (ONNX + all-MiniLM-L6-v2).

## Migration Strategy

- **Migrate:** CONSTRAINTs (~40 entries), DECISIONs (~15-20), COMMIT links (sparse)
- **Start fresh:** Implementation logs ("changed line 33"), "Pruned" sections, narrative "Changes" text
- **Archive:** `context.md` → `context.archive.md` (read-only human reference)
- **Ongoing:** `context.md` becomes a generated view via `memory_render()`, no longer source of truth

## Impact

| Metric | Before | After |
|--------|:------:|:-----:|
| Context loaded per session | ~36K tokens | ~3.5K tokens (selective) |
| Constraint recall | Position-dependent attention | Explicit `memory_by_type(CONSTRAINT)` |
| Cross-session continuity | Read entire history | Query relevant history |
| Role enforcement | Workflow text (advisory) | MCP validation (mechanical) |
| Multi-phase tracking | Scattered across files | Single `memory_search` call |

## Open Questions for Architecture Phase

1. Exact FTS5 tokenizer config (unicode61? porter stemmer?)
2. DB file location — per-project or global?
3. Backup/export strategy (git-tracked SQLite? periodic markdown dumps?)
4. Whether to namespace entries by project (multi-workspace support)
5. MCP transport: stdio (like Narsil) or HTTP?
6. Auth model for multi-agent scenarios (agent identifies role how?)
