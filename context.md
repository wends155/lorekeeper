# Project Context

## Current Status
- ✅ **Phases 0-G Complete**
- ✅ **Documentation Finalized** (README.md, justfile, spec.md, architecture.md, context.md)
- All technical debt from the initial audit has been remediated.
- The project is fully functional, lint-compliant, and well-tested.

## Recent Changes (2026-03-13)
- Refactored `src/server/mod.rs` to fix `items-after-test-module` and test panics (`unwrap`/`expect`).
- Aligned SDK traits (`on_error`, `ListToolsRequest`).
- Updated `spec.md` with missing trait methods (`get`, `stats`, `render_all`).
- Created `README.md` with installation, configuration, and feature overview.
- Created `justfile` with standard build, test, and lint recipes, configured for Windows PowerShell.

## Hard Constraints
1. **Zero-Warning Tolerance:** Must compile with `#![deny(clippy::all, clippy::pedantic)]`.
2. **Error Handling:** Avoid `.unwrap()`, `.expect()`, and `panic!()`. Handle gracefully or propagate setup errors via `Result`. `anyhow` is strictly forbidden.
3. **Data Loss:** Soft-delete (`is_deleted=true`) rather than hard-delete to preserve history.
4. **Toolchain Limitations:** Do NOT use chaining, redirects, or pipeline operators within `pwsh` for `SafeToAutoRun` commands (due to IDE interference). One discrete command per call. Use native tools where available.
5. **NARSIL Configuration:** Before querying Narsil in a new session, verify the index paths. If it references old locations, ask the user to restart Narsil.
