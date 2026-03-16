# Project Context

## Current Status
- ✅ **Phases 0-G Complete**
- ✅ **Documentation Finalized** (README.md, justfile, spec.md, architecture.md, context.md)
- All technical debt from the initial audit has been remediated.
- The project is fully functional, lint-compliant, and well-tested.

### [x] Release v0.3.0 (2026-03-16)
- Implemented multi-project support via the `lorekeeper_set_root` tool.
- Refactored `LoreHandler` to use `RwLock` for dynamic database switching at runtime without server restart.

### [x] Release v0.2.0 (2026-03-16)
- Upgraded `rust-mcp-sdk` from `0.8.3` → `0.9.0` for MCP protocol `2025-06-18`/`2025-11-25` compatibility.
- Added `InitializeRequest`/`PingRequest` handlers to `ServerHandlerCore` (v0.9.0 dispatch change).
- Synced MSRV to `1.94.0` across README, architecture.md, and Cargo.toml.
- Updated MCP config examples with `LOREKEEPER_ROOT` environment variable documentation.

### [x] Upgrade rust-mcp-sdk to v0.9.0 (2026-03-16)
- Resolved MCP protocol version mismatch (Antigravity sent `2025-06-18`, SDK v0.8.3 only supported `2025-03-26`).
- Bumped `rust-mcp-sdk` to `0.9` (supports protocol `2025-11-25` with backward compatibility).
- Fixed `ToolInputSchema::new` parameter type change (`HashMap` → `BTreeMap`).
- Reinstalled Lorekeeper binary locally.

### [x] Crates.io Readiness & Publication Remediation (2026-03-16)
- Audited project for `cargo publish` readiness and remediated 6 findings.
- Removed `publish = false`, added required metadata (description, keywords, repository, etc.), and synced `rust-version`.
- Configured crate hygiene by excluding `.agent/` and internal documentation via `Cargo.toml`.
- Ensured zero-warning test suite under `cargo package --list` by configuring `#[allow(clippy::expect_used)]` on tests.

### [x] Builtin Help System Resync (2026-03-16)
- Remediated 8 documentation findings from the Auto-Evolution audit.
- Synced `help.rs` and `main.rs` to include `SESSION_SUMMARY`, `lorekeeper_reflect`, and access/duplicate tracking.
- Verified 100% compliance gate (zero fmt/lint/test/doc warnings).

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
