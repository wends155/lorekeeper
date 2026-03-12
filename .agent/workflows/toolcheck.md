---
description: Validate tooling environment at session start (Session Bootstrap)
---

// turbo-all

# Toolcheck Workflow

This workflow validates the agent's tooling environment at session start.
It checks MCP connectivity, repo indexing, toolchain health, workflow/script
availability, and identifies automation opportunities.

> [!IMPORTANT]
> Run this workflow at the start of every session to ensure all tools are
> operational before beginning work.

## Trigger

User invokes: `/toolcheck`

## Steps

### 1. Environment Scan

Run the following checks. All are auto-runnable:

**Shell Tools:**
// turbo
- `git --version`
// turbo
- `rg --version`

**Git Non-Interactive Safety:**
// turbo
- `git config credential.helper`

If the output matches `manager`, `wincred`, or `osxkeychain`, set these env vars for the session:
- `$env:GCM_INTERACTIVE = 'never'`
- `$env:GIT_TERMINAL_PROMPT = '0'`

**Rust Toolchain:**
// turbo
- `rustc --version`
// turbo
- `cargo --version`
// turbo
- `cargo clippy --version`
// turbo
- `rustfmt --version`
// turbo
- `rustup show`

**Workflow & Script Files:**
Use `find_by_name` to verify all expected `.md` files exist in `.agent/workflows/` and all expected `.ps1` files exist in `.agent/scripts/`.

**Project Detection:**
Use `view_file` on `Cargo.toml`, `package.json`, or `go.mod` (whichever exists in repo root).

Produce a structured report from the above output (see Session Readiness Report format in Step 6).

### 2. Diagnose & Fix

For each ❌ item in the scan report:

1. **Diagnose** — determine why the tool is missing or misconfigured.
   Common causes: not installed, wrong PATH order, missing component.

2. **Attempt user-space fix** *(if possible)*:

   | Issue | Fix |
   |-------|-----|
   | Missing clippy | `rustup component add clippy` |
   | Missing rustfmt | `rustup component add rustfmt` |
   | Missing rg | `scoop install ripgrep` (preferred) or `cargo install ripgrep` (slow, compiles from source) |
   | Scoop `link.exe` shadowing MSVC | Advise user to reorder PATH |
   | Missing workflow/script file | ⚠️ Cannot fix — warn user |


3. **Re-scan** — re-run the individual version commands from Step 1 to confirm the fix.

4. **If unfixable** — collect into warnings with:
   - What failed
   - Why it can't be auto-fixed (e.g., requires admin, requires manual install)
   - Recommended action for the user

> [!NOTE]
> Fixes are **user-space only** — no admin rights, no sudo.
> If a fix requires elevated privileges, warn the user and provide
> the exact command they would need to run manually.

### 3. MCP Connectivity

#### Narsil MCP

1. **Connectivity**: Call `list_repos` — if it returns, Narsil is connected.
2. **Repo validation**: Call `validate_repo` with the current project path.
3. **Indexing**: Call `reindex` to trigger a fresh index for the session.
4. **Status**: Call `get_index_status` to confirm indexing is complete and review enabled features (git, call-graph, persist, watch).
5. **Dynamic Scoping**: Note the `$RepoRoot` path from the environment scan summary. **You MUST use this path as the `path="..."` argument** in all subsequent Narsil tool calls to isolate your analysis to the current project, avoiding noise from the macro-workspace.

If Narsil is **not available**, note it as a warning — not a blocker.
Other workflows can fall back to manual investigation.

#### Sequential Thinking MCP

1. **Connectivity**: Call `sequentialthinking` with a simple diagnostic thought.
   - If it returns, Sequential Thinking is available.
   - If it errors, note as a warning.

#### Context7 MCP

1. **Connectivity**: Call `resolve-library-id` with a simple query.
   - If it returns, Context7 is available for documentation lookups.
   - If it errors, note as a warning.

### 4. Project Assessment

If **Narsil MCP** is connected and indexed, perform a project-level scan:

| Tool | Purpose |
|------|---------|
| `get_project_structure` | Understand repo layout and key files |
| `check_dependencies` | Scan for known vulnerable dependencies |
| `get_security_summary` | Overall security posture |

Report any critical vulnerabilities or structural issues found.

### 5. Automation Opportunities

If **Sequential Thinking MCP** is available, use `sequentialthinking` to analyze:

1. **Scan results** — are there patterns that could be automated?

2. **TODO/FIXME markers** — scan with:
// turbo
   `rg -n "TODO|FIXME|HACK" --type-add "code:*.{rs,go,ts,js,svelte,py}" --type code .`
3. **Project structure** — are there build scripts, CI configs, or Makefiles?
4. **MCP capabilities** — which Narsil tools could help with current project state?
5. **Script gaps** — are there repetitive tasks that need a new script?

If Sequential Thinking is **not available**, perform this reasoning inline.

### 6. Session Readiness Report

Produce the final structured report:

```markdown
## 🚀 Session Readiness Report

### Environment
| Tool | Status | Version/Details |
|------|--------|----------------|
| PowerShell | ✅/❌ | version (edition) |
| Git | ✅/❌ | version |
| Rust | ✅/❌ | version + edition |
| Linker | ✅/❌ | MSVC/GCC + conflict status |
| rg | ✅/❌ | version |

### MCP Servers
| Server | Status | Details |
|--------|--------|---------|
| Narsil | ✅/❌ | repos indexed, features enabled |
| Sequential Thinking | ✅/❌ | available/unavailable |

### Workflow Ecosystem
| Component | Status |
|-----------|--------|
| Workflows | N/M present |
| Scripts | N/M present |

### Fixes Applied
- [list of auto-fixes attempted and their results]

### ⚠️ Warnings
- [unfixable issues + recommended user actions]

### 🤖 Automation Opportunities
- [identified by Sequential Thinking analysis]
```

End with:

> ✅ **Session Ready.** All critical tools operational.

or:

> ⚠️ **Session Ready with warnings.** Review the warnings above.
> Non-critical issues documented — workflows will use fallback paths.

## Rules

1. **Always scan first** — never skip the environment scan (Step 1), even if "everything looks fine."
2. **Fix before warn** — attempt user-space fixes before escalating to the user.
3. **No admin** — all fixes must be user-space (rustup, cargo install, scoop).
4. **Always index** — trigger Narsil `reindex` for fresh data every session.
5. **Don't block** — unfixable issues are warnings, not blockers. Other workflows fall back to manual investigation.
6. **Report everything** — even passing items go in the report for the session record.
7. **Auto-run** — see `GEMINI.md` §6 Auto-Run table. All commands in this workflow are read-only; set `SafeToAutoRun: true` for every `run_command` call.
