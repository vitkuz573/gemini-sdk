# Coding Conventions

**Analysis Date:** 2026-08-08

## Naming Patterns

**Files:**
- Source files use lowercase with underscores: `client.rs`, `proto/parser.rs`.
- Test files use `<scope>_tests.rs`: `proto_tests.rs`, `integration_tests.rs`, `real_cookies.rs`.

**Functions:**
- Constructors use `from_*`/`with_*` prefixes: `from_cookie_header`, `with_language`, `with_max_retries`.
- Builder actions are prefixed with `with_` or use imperative verbs: `send_message`, `build_inner_req_list`, `extract_text_from_parsed_response`.
- Private helpers use `snake_case`: `extract_snlim0e`, `build_slot0`, `map_proto_state`.

**Variables:**
- Local variables use `snake_case`: `cookie_header`, `inner_req_list`, `continuation_token`.
- Acronyms that appear in the protocol are kept uppercase when they are domain terms: `WAA`, `OGADS`, `PSID`.

**Types:**
- Public structs/enums use PascalCase: `GeminiClient`, `ChatMessage`, `ModelCategory`, `Error`.
- Generic error type alias is `Result<T>` (`src/errors.rs:9`).

## Code Style

**Formatting:**
- `rustfmt.toml` configures edition 2021, max width 100, chain width 80, 4-space tabs, Unix newlines.
- Imports and modules are reordered (`reorder_imports = true`, `reorder_modules = true`).
- Field-init shorthand and try shorthand are enabled.

**Linting:**
- `cargo clippy --all-targets -- -D warnings` is the expected check.
- `src/lib.rs` enables `#![warn(missing_docs)]` and `#![warn(clippy::all)]` while allowing a broad list of pedantic lints (`clippy::pedantic` is allowed globally, then specific lints are allowed individually).

## Import Organization

**Order:**
1. `std` / core imports.
2. External crate imports (e.g., `reqwest`, `serde_json`, `tokio`).
3. `crate::` internal imports.
4. `super::*` inside inline test modules.

**Path Aliases:**
- No path aliases (`use crate::foo as bar`) are used.
- Nested modules are imported explicitly (`use crate::proto::slots::ConversationState`).

## Error Handling

**Patterns:**
- Use `crate::Result<T>` and `Error` everywhere; avoid panics in library code.
- `?` is used to propagate errors from `reqwest`, `serde_json`, and internal fallible functions.
- `unwrap_or_default` is acceptable for non-critical fallback values (e.g., empty body on error path).
- `Error::is_transient` centralizes retry eligibility (`src/errors.rs:74`).
- Builder-style constructors return `Result<Self>` for invalid config/cookies.

## Logging

**Framework:** `tracing`.

**Patterns:**
- Use `tracing::debug!` for diagnostics that may help debugging but are not warnings.
- Example: `debug!(error = %e, "WAA init chain failed; continuing without WAA token")` (`src/client.rs:484`).
- The library does not initialize a subscriber; examples call `tracing_subscriber::fmt::init()`.

## Comments

**When to Comment:**
- Every public item has a doc comment (`#![warn(missing_docs)]`).
- Complex protocol constants and slot indices are explained inline.
- Workarounds and reverse-engineered behavior are documented with context: "The captured sJBwce payload is..." (`src/proto/mod.rs:83`).

**JSDoc/TSDoc:**
- Not applicable; Rust doc comments (`///`) are used throughout.
- Doc tests appear in `src/auth.rs` and `src/lib.rs`.

## Function Design

**Size:**
- Functions are generally small (< 50 lines), but orchestration functions in `src/client.rs` exceed this (`stream_generate_raw` ~120 lines, `init_session` ~35 lines).
- Prefer extracting helpers; the parser already uses many small extraction functions.

**Parameters:**
- Builders accept `impl Into<String>` for ergonomic string arguments.
- Internal helpers accept concrete references (`&str`, `&[Value]`) rather than generics.
- Long parameter lists are allowed for protocol builders with `#[allow(clippy::too_many_arguments)]` (`src/proto/slots.rs:56`, `src/client.rs:570`).

**Return Values:**
- Fallible functions return `crate::Result<T>`.
- Builders return `Self` for chaining.
- Extractors return `Option<T>` when absence is normal, `Result<T>` when absence is an error.

## Module Design

**Exports:**
- `src/lib.rs` explicitly re-exports public items; not all module contents are public.
- Internal helpers are `pub(crate)` or private (`mod retry; mod session;`).
- `PreparedRequest` is `#[doc(hidden)]` public for benchmarks and advanced use (`src/lib.rs:87`).

**Barrel Files:**
- `src/proto/mod.rs` re-exports parser and slots items at the `proto` module level.
- No other barrel-style aggregation is used.

---

*Convention analysis: 2026-08-08*
