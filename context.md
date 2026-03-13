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

## Final State Assessment
Project achieves 100% test coverage for server handlers and is fully compliant with all architectural and coding standards. The persistent memory bank (Lorekeeper) is ready for production use by agentic workflows.

## Hard Constraints
1. **Zero-Warning Tolerance:** Must compile with `#![deny(clippy::all, clippy::pedantic)]`.
2. **Error Handling:** Avoid `.unwrap()`, `.expect()`, and `panic!()`. Handle gracefully or propagate setup errors via `Result`. `anyhow` is strictly forbidden.
3. **Data Loss:** Soft-delete (`is_deleted=true`) rather than hard-delete to preserve history.
4. **Toolchain Limitations:** Do NOT use chaining, redirects, or pipeline operators within `pwsh` for `SafeToAutoRun` commands (due to IDE interference). One discrete command per call. Use native tools where available.
5. **NARSIL Configuration:** Before querying Narsil in a new session, verify the index paths. If it references old locations, ask the user to restart Narsil.
