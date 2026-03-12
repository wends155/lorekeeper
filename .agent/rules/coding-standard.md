# Rust Coding Standards & Latest Practices

> **Applies to:** All Rust codebases within this workspace.
> **Last updated:** 2026-02-25

---

## 1. Version & Toolchain

| Item | Standard |
| :--- | :--- |
| **Rust Version** | Latest stable (currently **1.93.1**) |
| **Edition** | `2024` for all new projects |
| **Toolchain Manager** | `rustup` |
| **Required Components** | `rustfmt`, `clippy`, `rust-analyzer` |

> [!IMPORTANT]
> Pin `rust-version` in `Cargo.toml` to prevent accidental MSRV regressions.

---

## 2. Code Quality Gate (Zero-Exit Requirement)

Every PR / commit **must** pass all three gates before merge:

```sh
cargo fmt  --all -- --check   # Gate 1: Formatting
cargo clippy --all-targets --all-features -- -D warnings  # Gate 2: Linting
cargo test  --all-features    # Gate 3: Tests
```

| Metric | Target |
| :--- | :--- |
| Formatting | 100% `rustfmt` compliance |
| Linting | Zero `clippy` warnings (deny mode) |
| Documentation | 100% of public APIs documented |
| Test coverage | 100% of public functions tested |
| Benchmarks | Critical paths benchmarked with Criterion |

---

## 3. Project Configuration

### 3.1 `rustfmt.toml`

```toml
edition = "2024"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### 3.2 Clippy Configuration

Use the workspace-level `[lints]` table (Rust 1.74+) for consistent configuration
across all crates. The default `clippy::all` group already covers correctness and
common-style lints — that is **sufficient as the baseline**. Layer `pedantic` as
**warnings** for guidance, and cherry-pick high-value individual lints.

> [!TIP]
> Avoid blanket `deny` on `nursery` (lints are unstable and may change between
> releases) or `cargo` (situational, better handled per-project). Promote
> individual lints to `deny` only when the team has validated they don't produce
> false positives in the codebase.

```toml
# Cargo.toml (workspace root)
[workspace.lints.clippy]
# ── Baseline (default) ────────────────────────────────────────
all = "deny"                          # correctness + common style

# ── Guidance ──────────────────────────────────────────────────
pedantic = "warn"                     # stricter style — warn, don't block

# ── Cherry-picked high-value lints (deny) ─────────────────────
missing_errors_doc       = "deny"     # every Result-returning fn must document errors
missing_panics_doc       = "deny"     # every potentially-panicking fn must document it
undocumented_unsafe_blocks = "deny"   # enforce // SAFETY: comments
cast_possible_truncation = "deny"     # catch lossy integer casts
large_futures            = "deny"     # prevent accidentally huge futures on the stack

# ── Useful pedantic lints relaxed to allow (override as needed) ─
module_name_repetitions  = "allow"    # common in domain-driven designs
must_use_candidate       = "allow"    # too noisy for general use

[workspace.lints.rust]
unsafe_code              = "warn"     # highlight unsafe usage without hard-blocking

[lints]
workspace = true
```

#### Recommended Per-Project Additions

Enable these when they apply to your project:

| Lint | Level | When to enable |
| :--- | :--- | :--- |
| `clippy::nursery` | `warn` | Opt-in for experimental early warnings |
| `clippy::cargo` | `warn` | When publishing crates to crates.io |
| `clippy::missing_docs_in_private_items` | `warn` | For library-heavy projects needing internal docs |
| `clippy::unwrap_used` | `deny` | For production services (not tests) |
| `clippy::expect_used` | `warn` | Pair with `unwrap_used` for stricter error handling |
| `clippy::indexing_slicing` | `warn` | For safety-critical code avoiding panics |

---

## 4. Code Standards

### 4.1 Error Handling

> [!CAUTION]
> **No Crashes** — avoid all patterns that cause uncontrolled program termination:
> - `.unwrap()` / `.expect()` — reserved for tests and provably-infallible cases (with comment).
> - `panic!()` / `unreachable!()` — use error propagation instead. If truly unreachable, add a comment.
> - `todo!()` — never in production; use compile-time `#[cfg]` gates or return `Err(Unimplemented)`.
> - Index out-of-bounds (`vec[i]`) — use `.get(i)` and handle `None`.

**`thiserror` vs `anyhow`:**
- **`thiserror`** — for library error types. Define structured, matchable error enums with `#[derive(Error)]`. Each variant gets a descriptive `#[error("...")]` message. Use `#[from]` for transparent conversions from source errors.
- **`anyhow`** — for application-level error propagation. Use `anyhow::Result` when you don't need callers to match on specific variants. Add context with `.context("what failed")`.

**Error Handling Checklist:**

- [ ] Every error communicates **what**, **where**, and **why**.
- [ ] No silent failures — all `Result` values are propagated or logged.
- [ ] Errors are contextually wrapped at module boundaries.
- [ ] `#[from]` is used for transparent conversions; manual `map_err` for added context.

---

### 4.2 Async / Await Best Practices

Wrap external I/O in `tokio::time::timeout`. Chain `.map_err()` to convert timeout and network errors into domain error types. Always propagate errors with `?` rather than swallowing them.

**Async Rules:**

- Prefer `tokio` as the async runtime for all server-side work.
- Always set timeouts on external I/O (network, file, IPC).
- Use `tokio::select!` for concurrent branch cancellation, **not** manual `JoinHandle` polling.
- Avoid `block_on` inside async contexts — it will deadlock the runtime.
- Use structured concurrency (`JoinSet`, `TaskTracker`) over raw `tokio::spawn`.

---

### 4.3 Memory Management & Ownership

**Ownership Rules:**

- Prefer borrowing (`&T`, `&mut T`) over cloning.
- Use `Arc` only when shared ownership across threads is required; prefer `Rc` in single-threaded code.
- For shared mutable caches, use `Arc<RwLock<HashMap>>` or `dashmap`.
- Use `Cow<'_, str>` when a function may or may not need to allocate.
- Avoid `Box<dyn Trait>` when generics (`impl Trait` or `<T: Trait>`) suffice.
- Use `Arc<[T]>` instead of `Arc<Vec<T>>` for immutable shared slices.

---

### 4.4 Type System & API Design

- **Newtype pattern**: Wrap primitive types to add semantic meaning and prevent misuse. Validate in the constructor, return `Result`. Once constructed, the type guarantees validity.
- **Builder pattern**: Use for structs with many optional fields (see § 4.6.1 for full guidance).
- **Typestate pattern**: Encode valid state transitions in the type system (see § 4.6.2).
- **`#[must_use]`**: Apply to functions whose return value should not be silently discarded.
- **`#[non_exhaustive]`**: Apply to public enums and structs that may grow.
- **Sealed traits**: Use the sealed-trait pattern for traits not intended for external implementation.
- See § 4.6 for the complete design patterns reference and selection guide.

---

### 4.5 Documentation Standards

#### Function & Type Docs

Every public item **must** have a doc comment (`///`) that includes:

1. **Summary** — one-line description.
2. **Details** — extended explanation (if needed).
3. **`# Errors`** — documents each error variant the function can return.
4. **`# Panics`** — documents conditions under which the function panics (ideally none).
5. **`# Examples`** — runnable code example (serves as a doc-test).

#### Module Docs

Every `lib.rs`, `main.rs`, and top-level module file **must** have a `//!` doc comment providing:

- **Purpose** — what this module/crate does.
- **Key types** — the primary structs, traits, and enums it exposes.
- **Usage** — brief guidance on how consumers should use it.

---

### 4.6 Design Patterns & Best Practices

Rust's type system enables patterns that catch entire categories of bugs at compile time.
This section codifies the patterns we use most, with guidance on when and why to apply each.

#### 4.6.1 Builder Pattern

Use when constructing structs with many fields — especially when some are optional, have
defaults, or require validation.

Prefer the `bon` or `typed-builder` crates for derive-based builders. Fall back to a manual builder only when you need custom validation logic during construction. Manual builders follow the pattern: optional fields stored as `Option<T>`, chainable setter methods returning `Self`, and a `build()` method that validates and returns `Result<T, ValidationError>`.

---

#### 4.6.2 Typestate Pattern

Encode protocol steps or lifecycle phases into the type system so that **invalid state
transitions are compile-time errors**. Use zero-sized marker types as generic parameters — no runtime cost. Each state gets its own `impl` block, so only valid operations are available at each phase.

**When to use:**
- Protocol handshakes (connect → auth → ready).
- Build pipelines (configure → validate → execute).
- File I/O (open → write → flush → close).

---

#### 4.6.3 RAII & Drop Guards

Use Rust's `Drop` trait to guarantee resource cleanup runs automatically when a value goes out of scope — even on early returns or panics. The guard pattern: hold a reference to the resource plus a `committed` flag; on drop, if uncommitted, perform cleanup (e.g., rollback).

**Common RAII use cases:**

| Resource | Guard / Type | Cleanup Action |
|:---|:---|:---|
| Database transaction | `TransactionGuard` | Rollback on drop |
| Temp file / directory | `tempfile::TempDir` | Delete on drop |
| Mutex lock | `MutexGuard` | Release on drop |
| Timer / span | `tracing::span::Entered` | Record elapsed on drop |
| File lock | `fs2::FileLock` | Release on drop |

> [!TIP]
> For ad-hoc guards without a dedicated struct, use the `scopeguard` crate (`defer! { cleanup(); }`).

---

#### 4.6.4 Extension Traits

Add domain-specific methods to types you don't own (e.g., `std`, `serde_json`) without violating the orphan rule. Define a trait, implement it for the foreign type, and re-export it.

**Rules:**
- Name the trait `<Type>Ext` (e.g., `StrExt`, `ResultExt`, `IteratorExt`).
- Keep extension traits in a dedicated `ext` module.
- Re-export from the crate prelude if they're used widely.

---

#### 4.6.5 Interior Mutability

Use interior mutability when you need to mutate data behind a shared reference (`&T`).
Choose the narrowest primitive that satisfies your requirements:

| Type | Thread-safe? | Checked at | Use when |
|:---|:---|:---|:---|
| `Cell<T>` | No | Compile time | `T: Copy`, single thread, simple swap/replace |
| `RefCell<T>` | No | Runtime | Single thread, need `&mut T` borrows |
| `Mutex<T>` | Yes | Runtime | Multi-thread, exclusive write access |
| `RwLock<T>` | Yes | Runtime | Multi-thread, many readers / rare writers |
| `OnceLock<T>` | Yes | Runtime | Write-once lazy initialization |
| `Atomic*` | Yes | Lock-free | Counters, flags, simple numeric state |

> [!CAUTION]
> Prefer `OnceLock` (std) or `LazyLock` over hand-rolled `Mutex<Option<T>>`
> for lazy initialization. It's safer and more readable.

---

#### 4.6.6 Enum Dispatch vs Trait Objects

Choose between compile-time (`enum`) and runtime (`dyn Trait`) polymorphism based on
whether the set of variants is **closed** or **open**.

| Criterion | Enum dispatch | Trait objects (`dyn Trait`) |
|:---|:---|:---|
| Variant set | Closed (known at compile time) | Open (extensible by consumers) |
| Performance | Monomorphized, inlineable | Vtable indirection |
| Pattern matching | Exhaustive `match` | Not available |
| Object safety needed? | No | Yes |
| Binary size | Larger (monomorphization) | Smaller |

Use enum dispatch for closed sets of known variants. Use trait objects for open, plugin-style extensibility. When the set is closed but you want trait syntax, consider the `enum_dispatch` crate.

---

#### 4.6.7 Repository Pattern

Centralize data access behind a trait (e.g., `trait UserRepo`). Modules depend on the trait, not the database directly. This prevents scattered DB access across modules, makes testing trivial (mock the trait), and makes storage changes a single-point refactor.

**Rules:**
- One trait per aggregate root or domain entity.
- Methods return domain types, not raw DB rows.
- Implementations live in a dedicated `infra` or `persistence` module.
- Errors are domain errors, not raw `sqlx::Error` — map at the boundary.

---

#### 4.6.8 Dependency Injection via Traits

The general principle that makes Repository and other patterns testable. Define behavior as a trait, accept `impl Trait` or `&dyn Trait` in struct constructors. In production, pass the real implementation. In tests, pass a mock (`mockall::automock` or a manual stub).

**Applies to:**
- Database access (Repository)
- HTTP clients
- File system operations
- Email/notification senders
- Clock/time providers

**Rules:**
- Define the trait in the consuming module (not the implementing module).
- Use `#[mockall::automock]` on the trait for automatic mock generation.
- Prefer `impl Trait` in function args, `Box<dyn Trait>` or generics in struct fields.

---

#### 4.6.9 Pattern Selection Guide

| Problem | Recommended Pattern | Ref |
|:---|:---|:---|
| Many optional constructor fields | Builder | § 4.6.1 |
| Compile-time state machine enforcement | Typestate | § 4.6.2 |
| Guaranteed resource cleanup | RAII / Drop Guard | § 4.6.3 |
| Add methods to types you don't own | Extension Trait | § 4.6.4 |
| Mutation behind `&T` | Interior Mutability | § 4.6.5 |
| Known, closed set of behaviors | Enum Dispatch | § 4.6.6 |
| Open, extensible set of behaviors | Trait Objects (`dyn`) | § 4.6.6 |
| Prevent primitive type misuse | Newtype | § 4.4 |
| Restrict external trait implementations | Sealed Trait | § 4.4 |
| Scattered data access across modules | Repository | § 4.6.7 |
| Testable external dependencies | DI via Traits | § 4.6.8 |

---

### 4.7 Module & Workspace Organization

Rules for structuring multi-module, multi-crate Rust projects.

#### 4.7.1 Visibility

Minimize public surface area. Default to private; widen only when needed.

| Visibility | Use when |
|:---|:---|
| (private) | Implementation detail — no outside access needed |
| `pub(super)` | Parent module needs access (e.g., test helpers) |
| `pub(crate)` | Other modules within the same crate need access |
| `pub` | Part of the crate's public API, documented and stable |

> [!CAUTION]
> **Never** make something `pub` just to satisfy a compiler error. If a type is needed
> across crate boundaries, re-evaluate the crate boundary — it may be in the wrong place.

---

#### 4.7.2 Workspace Organization

| Criterion | Same crate (modules) | Separate crate |
|:---|:---|:---|
| Shared types/traits | Same crate | Extract a `*-core` or `*-types` crate |
| Independent release cycle | N/A | Separate crate |
| Build time isolation | Same crate | Separate crate (parallel compilation) |
| Feature-gated functionality | Feature flags within crate | Separate crate |
| Shared test utilities | `#[cfg(test)]` module | Dedicated `*-test-utils` crate |

**Conventions:**
- Name crates `<project>-<role>` (e.g., `tars-core`, `tars-mcp`, `tars-cli`).
- Put shared types in a `-core` or `-types` crate that others depend on.
- Never create circular dependencies between crates.

---

#### 4.7.3 Cross-Crate Error Strategy

Each crate defines its own error type with `thiserror`. At crate boundaries, convert foreign errors into local error variants using the `#[from]` and `map_err` rules from §4.1.

**Rules:**
- Never expose internal crate error types in your public API — wrap them.
- For workspaces: a `-core` crate may define shared error types that all crates use as a base.

---

#### 4.7.4 Re-exports & Prelude

Curate the public API surface from `lib.rs` using `pub use`:

**Rules:**
- Re-export the primary types, traits, and error types from `lib.rs`.
- Create a `prelude` module for types users will import in nearly every file.
- Keep prelude small — only the most frequently used items.
- Internal modules should be `pub(crate)`, not `pub`, unless they're part of the API surface.

---

#### 4.7.5 Feature Flags

Use Cargo features for optional functionality within a crate:

**Rules:**
- Default features should cover the common use case.
- Feature names should be descriptive: `serde`, `async`, `cli`, not `feat1`.
- Gate expensive dependencies behind features (e.g., `tracing`, `serde`).
- Document all features in `Cargo.toml` with inline comments.
- Test with `--all-features` AND with no features to catch compilation issues.

---

#### 4.7.6 Module Communication

Modules communicate through well-defined interfaces, not by reaching into each other's internals. Apply DI via Traits (§ 4.6.8) for all cross-module dependencies.

**Rules:**
- Data flows through shared types from a `-core` crate, not through direct module imports.
- If module A needs to call module B, A depends on B's **trait**, not B's **struct**.
- Use events or channels for decoupled communication between modules that shouldn't know about each other.

---

### 4.8 Observability & Logging

Use `tracing` as the standard logging and instrumentation framework for all Rust projects.

#### 4.8.1 Log Levels

| Level | Use for | Example |
|:---|:---|:---|
| `error!` | Actionable failures requiring attention | DB connection lost, auth failure |
| `warn!` | Recoverable issues, degraded behavior | Retry succeeded, fallback used |
| `info!` | Milestones, lifecycle events | Server started, request completed |
| `debug!` | Internal state useful during development | Cache hit/miss, parsed config values |
| `trace!` | Verbose flow, hot-path details | Function entry/exit, loop iterations |

#### 4.8.2 Structured Fields

Always use structured key-value fields, not string interpolation:

- Use `%` for `Display` formatting: `tracing::info!(user_id = %id, "processing")`
- Use `?` for `Debug` formatting: `tracing::debug!(config = ?cfg, "loaded")`
- Use typed fields for metrics: `tracing::info!(duration_ms = elapsed.as_millis(), "completed")`

#### 4.8.3 Spans & Instrumentation

- Use `#[tracing::instrument]` on public functions to auto-create spans with arguments.
- Skip sensitive fields: `#[instrument(skip(password, token))]`.
- **Spans** track duration and context of an operation (a unit of work with a start and end).
- **Events** (`info!`, `error!`) are point-in-time occurrences within a span.
- Nest spans to build a call tree — each span inherits its parent's context.

#### 4.8.4 Subscriber Setup

- Use `tracing-subscriber` with `EnvFilter` for runtime-configurable log levels.
- For JSON output (production): `tracing_subscriber::fmt().json()`.
- For human output (development): `tracing_subscriber::fmt().pretty()`.
- For OpenTelemetry integration: add `tracing-opentelemetry` as a layer.

**Rules:**
- Never use `println!`, `eprintln!`, or `dbg!` in production code — use `tracing` macros.
- Every public async function should have `#[instrument]`.
- Include request/correlation IDs in spans for distributed tracing.
- Log at `info` level for operations that help reconstruct what happened in production.

---

### 4.9 Defensive Programming

Validate inputs, enforce invariants, and handle edge cases — don't assume callers will provide valid data.

**Rules:**
- Validate all public function inputs at the boundary. Return `Err` for invalid data — don't propagate it deeper.
- Use newtypes (§ 4.4) to encode validation in the type system so invalid states are unrepresentable.
- Handle all edge cases explicitly: empty collections, zero-length strings, `None`, boundary values (0, `MAX`, `MIN`).
- Prefer `.get()` over indexing (`[]`) for slices, maps, and vectors.
- Use `saturating_*` or `checked_*` arithmetic to prevent overflow/underflow.
- Fail gracefully — return meaningful errors rather than crashing. A degraded response is better than no response.

---

### 4.10 Environment Configuration

Centralize configuration loading and validation to prevent runtime surprises.

#### File Hierarchy

| File | Committed? | Purpose |
|:---|:---:|:---|
| `.env.example` | ✅ Yes | Template with all keys, placeholder values, and comments |
| `.env` | ❌ No (gitignored) | Local development values — real API keys for dev |
| `.env.test` | ⚠️ Optional | Test-safe values (no real API keys, localhost URLs) |
| Production | N/A | Real env vars injected by deployment platform |

> [!IMPORTANT]
> `.env.example` must be committed and kept up-to-date. It serves as documentation
> for every configuration key the application requires. Never put real secrets in it.

#### Config Struct Pattern

All configuration must flow through a typed, validated struct:

- Parse from environment at application startup using `dotenvy` + `std::env::var`
- Fail fast — if a required variable is missing, the application must exit immediately with a
  clear error message naming the missing variable
- Use newtypes (§ 4.4) for validated config values (e.g., `DatabaseUrl`, `ApiKey`, `Port`)
- Use an `Environment` enum (`Dev`, `Staging`, `Prod`) to control behavior differences

**Rules:**
- Never scatter `std::env::var()` calls throughout the codebase — read all env vars once into
  the config struct at startup
- Never use `Option<T>` for required configuration — if it's required, fail at startup, not at
  first use
- Never hardcode URLs, ports, or credentials — all external endpoints come from config
- Load `.env` only in dev/test — production uses real env vars
- Use `#[cfg(test)]` or a test-specific config constructor for test environments

#### Gitignore Requirements

Every project with environment configuration must include in `.gitignore`:

```
.env
.env.local
.env.*.local
```

> [!CAUTION]
> If `.env` is accidentally committed, rotate ALL secrets immediately.
> `git rm --cached .env` removes it from tracking, but the secrets are
> already in git history.

---

## 5. Testing Standards

### 5.1 TDD Flow

Follow the **Red → Green → Blue** cycle:

1. **Red** — Write a failing test that specifies the desired behavior.
2. **Green** — Write the minimum code to make the test pass.
3. **Blue** — Refactor for clarity and performance while keeping tests green.

### 5.2 Unit Tests

Tests follow the **Arrange-Act-Assert** pattern. Co-locate unit tests in a `#[cfg(test)] mod tests` block within the same file. Use `#[tokio::test]` for async tests. Name tests descriptively: `<action>_<scenario>_<expected>` (e.g., `process_invalid_format_returns_error`).

### 5.3 Integration Tests

Place integration tests in a top-level `tests/` directory. Each file in `tests/` is compiled as a separate crate — it can only access the public API.

**General Rules:**
- One file per feature area (e.g., `tests/auth.rs`, `tests/pipeline.rs`).
- Use shared fixtures via a `tests/common/mod.rs` helper module.
- Integration tests should exercise real module interactions, not mock everything.
- For tests requiring external services (DB, HTTP), use `testcontainers` or `wiremock`.

---

#### 5.3.1 Database Tests (Testcontainers)

Use `testcontainers` to spin up ephemeral database containers for integration tests. Each test suite gets a fresh, isolated database — no shared state, no dependency on local infrastructure.

**Pattern — `TestDb` shared fixture:**

Create a reusable `TestDb` struct in `tests/common/mod.rs` that manages the container lifecycle:
- Start a Postgres container via `testcontainers::runners::AsyncRunner`
- Run migrations automatically (via `sqlx::migrate!()` or `refinery`)
- Provide a connection pool to the test
- Container is dropped (and destroyed) when the test ends

**Rules:**
- Each test suite gets its own container — no sharing between test files
- Migrations run before every suite — tests always start with a known schema
- Never depend on local `docker compose up` for tests — tests must be self-contained
- Use `#[tokio::test]` for async database tests
- Keep a `docker-compose.yml` in the project root for **local development only** (committed, documented in `architecture.md`)

**Docker Compose (for local dev only, not tests):**

```yaml
# docker-compose.yml — local development services
services:
  postgres:
    image: postgres:17
    environment:
      POSTGRES_DB: myapp_dev
      POSTGRES_USER: dev
      POSTGRES_PASSWORD: dev
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

> [!CAUTION]
> `docker-compose.yml` is for developer convenience — `cargo test` must never require
> `docker compose up`. If tests need a database, they spin up their own via testcontainers.

---

#### 5.3.2 External API Tests (Wiremock)

Use `wiremock` to simulate external HTTP APIs in integration tests. This enables testing
HTTP interaction patterns (headers, status codes, timeouts) without calling live services.

**Three-Layer Testing Strategy:**

| Layer | Tool | What It Tests | When to Use |
|:---|:---|:---|:---|
| **Unit** | `mockall` | Business logic in isolation | Always — every function |
| **Integration** | `wiremock` | HTTP request/response contracts | When code makes HTTP calls |
| **E2E / Sandbox** | Real API (dev endpoint) | Full end-to-end integration | Manual / staging only |

**Pattern — Wiremock MockServer:**

- Create a `wiremock::MockServer::start()` in the test setup
- Mount response mocks for expected endpoints
- Pass the mock server's URI to the client under test (via config or constructor)
- Assert that the mock was called with expected request properties

**Response Scenarios to Cover:**

| Scenario | Response | Why It Matters |
|:---|:---|:---|
| Success | 200 + valid JSON body | Happy path |
| Client error | 400/422 + error body | Input validation feedback |
| Auth failure | 401/403 | Token expiry, permission checks |
| Not found | 404 | Missing resource handling |
| Rate limit | 429 + `Retry-After` header | Backoff logic |
| Server error | 500/503 | Retry and fallback behavior |
| Timeout | No response (delay) | Timeout handling and circuit breaking |

**Rules:**
- Every external API must be behind a trait (§ 4.6.8) — the trait enables both mockall
  (unit) and wiremock (integration) testing
- Integration tests must cover at least: success, error response, and timeout scenarios
- Never call live external APIs in CI — all integration tests use wiremock
- Document the response contract (status codes, headers, body shape) in the trait's doc comment
- Use `wiremock::matchers` to verify request method, path, headers, and body — not just the response

> [!TIP]
> For APIs with complex auth flows (OAuth, JWT), create a dedicated `MockAuthServer`
> test fixture that handles token issuance and validation.

---

#### 5.3.3 Test Environment Configuration

Separate test configuration from production configuration to prevent accidental
use of real credentials in tests.

**Rules:**
- Create a test-specific config constructor (e.g., `AppConfig::for_test()`) that uses safe defaults
- Use `.env.test` for integration test environment variables when needed
- Test timeouts should be shorter than production (e.g., 5s vs 30s) to catch slow tests early
- Test databases use either testcontainers (preferred) or a `_test`-suffixed database name
- Never use production API keys in test config — use wiremock or sandbox keys
- Load test config via `#[cfg(test)]` module or a test helper function

### 5.4 Property-Based Tests

Use `proptest` for invariant checking on complex transformations. The test generates random inputs matching a pattern and asserts that invariants hold for all of them. Acceptable error variants should be explicitly matched; unexpected errors fail the test.

### 5.5 Testing Checklist

- [ ] Every function has at least one happy-path and one error-path test.
- [ ] Async functions are tested with `#[tokio::test]`.
- [ ] Edge cases (empty input, max values, unicode) are covered.
- [ ] Property-based tests exist for complex transformations.
- [ ] Integration tests cover cross-module interactions.
- [ ] Mocks (`mockall`) are used for external dependencies (see § 4.6.8).
- [ ] Doc-tests compile and pass (`cargo test --doc`).
- [ ] External APIs are tested with wiremock (HTTP) or mockall (trait) — not called live in CI.
- [ ] Database tests use testcontainers or isolated test databases.
- [ ] `.env.example` is committed and up-to-date with all required keys.

---

## 6. Performance Benchmarking

Use **Criterion** for all performance-sensitive code paths. Use `criterion::black_box` to prevent the optimizer from eliding work. For async benchmarks, use `.to_async(&runtime)` on the bencher. Benchmark critical paths and track regressions against baselines.

---

## 7. CI/CD Integration

CI/CD pipeline configuration is project-specific. Define it in `architecture.md § Toolchain`.
At minimum, the pipeline must enforce the Code Quality Gate (§2).

---

## 8. Tools & Technologies

### Development

| Tool | Purpose |
| :--- | :--- |
| `rustup` | Toolchain management |
| `rustfmt` | Code formatting |
| `clippy` | Linting & code analysis |
| `rust-analyzer` | Language server / IDE support |
| `cargo` | Build, test, package management |

### Testing & Benchmarking

| Tool | Purpose |
| :--- | :--- |
| `cargo test` | Built-in unit & integration tests |
| `criterion` | Statistical benchmarking |
| `proptest` | Property-based / fuzz testing |
| `mockall` | Mocking framework |
| `testcontainers` | Ephemeral Docker containers for integration tests |
| `wiremock` | HTTP API mocking for integration tests |
| `cargo-tarpaulin` | Code coverage |

### Infrastructure

| Tool | Purpose |
| :--- | :--- |
| `docker-compose` | Local development services (Postgres, Redis, etc.) |
| `dotenvy` | `.env` file loading |
| `sqlx` | Async SQL with compile-time checked queries |
| `refinery` | Database migration management |

### Quality & Security

| Tool | Purpose |
| :--- | :--- |
| `cargo audit` | Security vulnerability scanning |
| `cargo outdated` | Dependency staleness check |
| `cargo tree` | Dependency graph visualization |
| `cargo expand` | Macro expansion debugging |
| `cargo deny` | License & advisory policies |

---

## 9. Metrics & Monitoring

### Code Quality Metrics

| Metric | Target | Tool |
| :--- | :--- | :--- |
| Test coverage | >= 90% line coverage | `cargo-tarpaulin` |
| Clippy warnings | 0 | `cargo clippy` |
| Doc coverage | 100% public API | `cargo doc` |
| Benchmark regressions | < 5% | Criterion |

> [!NOTE]
> Line coverage target (>=90%) complements §2's requirement for 100% public API
> function coverage. Both apply — every public function must be tested, and
> overall line coverage should exceed 90%.

### Development Metrics

| Metric | Purpose |
| :--- | :--- |
| Build time | Track incremental & clean build perf |
| Dependency count | Minimize supply-chain surface |
| Security advisories | Zero unmitigated CVEs |

---

## 10. Quick Reference – Prohibited Patterns

| Don't | Do Instead |
| :--- | :--- |
| `.unwrap()` in production | Use `?`, `map_err`, or `.unwrap_or_default()` |
| `println!` for logging | Use `tracing::info!` / `tracing::error!` |
| `clone()` without reason | Borrow first; clone only when ownership is needed |
| Raw `thread::spawn` | Use `tokio::spawn` with structured concurrency |
| `unsafe` without comment | Add `// SAFETY:` explaining the invariant |
| Magic numbers | Named constants or enums |
| Wildcard imports `use foo::*` | Explicit imports or re-exports |
| Mutable globals | `OnceLock`, DI, or runtime config |
| Boolean flags for state tracking | Typestate pattern (§ 4.6.2) |
| Manual resource cleanup calls | RAII / Drop guard (§ 4.6.3) |
| `struct` with 10+ constructor args | Builder pattern (§ 4.6.1) |
| Wrapper structs for one method | Extension trait (§ 4.6.4) |
| `Mutex<Option<T>>` for lazy init | `OnceLock` or `LazyLock` (§ 4.6.5) |
| Direct DB access in business logic | Repository pattern (§ 4.6.7) |
| Hard-coded dependencies | DI via Traits (§ 4.6.8) |
| Scattered `std::env::var` calls | Centralized config struct (§ 4.10) |
| Calling live APIs in CI tests | Wiremock or mockall (§ 5.3.2) |
| Shared test database state | Testcontainers per suite or transaction rollback (§ 5.3.1) |
| `.env` committed to git | `.env.example` only; `.env` in `.gitignore` (§ 4.10) |

---

> [!NOTE]
> `todo!()` remains prohibited. For multi-phase projects, use `// STUB(Phase N): description`
> markers instead (see `phase-rules.md §3`). Stubs must be functional code that returns Ok
> and logs a warning — never panicking placeholders.

---

> **Maintained by:** The Architect role (High-Reasoning Model)
> **Compliance:** All code contributions are validated against this document during the Reflect phase.
