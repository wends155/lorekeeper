# Project Context

## Current Status
- ✅ **Phases 0-G Complete**
- ✅ **Documentation Finalized** (README.md, justfile, spec.md, architecture.md, context.md)
- All technical debt from the initial audit has been remediated.
- The project is fully functional, lint-compliant, and well-tested.

### [x] Full Compliance Audit & Remediation (2026-03-13)
- Conducted full project audit against `coding-standard.md` and `architecture.md`.
- Triaged 80 Narsil security findings (all FPs).
- Remediated 3 documentation gaps: fixed `architecture.md`, aligned `rustfmt.toml`, and expanded `lib.rs` docs.
- Verified 100% compliance gate (zero fmt/lint/test/doc warnings).

### [x] Exhaustive Path Coverage (2026-03-16)
- Implemented 26 new test cases achieving ~97%+ path coverage.
- Created `NoOpMcpServer` stub for async server handler verification.
- Covered all repository error paths, SQLite soft-delete guards, and state transition edge cases.
- Final test suite: 89/89 passing.

### [x] Documentation & MCP Installation Audit Remediation (2026-03-16)
- Installed `lorekeeper` binary to local PATH via `cargo install --path .`.
- Fixed tool count inaccuracies (11 -> 10) in `README.md` and `architecture.md`.
- Improved MCP configuration documentation for Antigravity and Claude Desktop.

## Final State Assessment
Project achieves ~97%+ path coverage across all modules and is fully compliant with all architectural and coding standards. The persistent memory bank (Lorekeeper) is robust, handles all error edge cases gracefully, and is ready for production use by agentic workflows.

## Hard Constraints
1. **Zero-Warning Tolerance:** Must compile with `#![deny(clippy::all, clippy::pedantic)]`.
2. **Error Handling:** Avoid `.unwrap()`, `.expect()`, and `panic!()`. Handle gracefully or propagate setup errors via `Result`. `anyhow` is strictly forbidden.
3. **Data Loss:** Soft-delete (`is_deleted=true`) rather than hard-delete to preserve history.
4. **Toolchain Limitations:** Do NOT use chaining, redirects, or pipeline operators within `pwsh` for `SafeToAutoRun` commands (due to IDE interference). One discrete command per call. Use native tools where available.
5. **NARSIL Configuration:** Before querying Narsil in a new session, verify the index paths. If it references old locations, ask the user to restart Narsil.
