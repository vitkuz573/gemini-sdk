# Codebase Structure

**Analysis Date:** 2026-08-08

## Directory Layout

```
[project-root]/
├── src/                  # Library source code
│   ├── proto/            # WIZ protocol helpers
│   │   ├── mod.rs        # Body builders and shared constants
│   │   ├── slots.rs      # 97-slot StreamGenerate request list
│   │   └── parser.rs     # Response parsing
│   ├── lib.rs            # Crate root and re-exports
│   ├── client.rs         # GeminiClient, ChatBuilder, session init
│   ├── chat.rs           # Chat types, Conversation, PreparedRequest
│   ├── auth.rs           # Cookie parsing, Credentials, SAPISIDHASH
│   ├── errors.rs         # Error enum and Result alias
│   ├── models.rs         # ModelCategory and ModelInfo
│   ├── session.rs        # SessionState extraction from /app HTML
│   ├── upload.rs         # Resumable image upload
│   ├── retry.rs          # Exponential backoff wrapper
│   └── attestation.rs    # Optional browser CDP attestation
├── tests/                # Integration and protocol tests
│   ├── fixtures/         # Captured/minimal response fixtures
│   ├── integration_tests.rs
│   ├── proto_tests.rs
│   └── real_cookies.rs   # Live integration tests (skipped without cookies)
├── examples/             # Usage examples
│   ├── text_chat.rs
│   ├── image_chat.rs
│   ├── stream_chat.rs
│   ├── test_attestation.rs
│   └── capture_fixtures.rs
├── benches/              # Criterion benchmarks
│   └── slot_building.rs
├── docs/                 # Human-readable documentation
│   └── protocol.md
├── Cargo.toml            # Package manifest
├── Cargo.lock            # Dependency lockfile
├── rustfmt.toml          # rustfmt configuration
├── clippy.toml           # clippy configuration
├── README.md             # Project overview
└── CONTRIBUTING.md       # Contribution guidelines
```

## Directory Purposes

**`src/`:**
- Purpose: All library implementation.
- Contains: 11 Rust modules (including `proto` submodule).
- Key files: `src/lib.rs`, `src/client.rs`, `src/chat.rs`, `src/proto/parser.rs`, `src/auth.rs`.

**`src/proto/`:**
- Purpose: Isolate WIZ protocol knowledge from the public API.
- Contains: request body builders, 97-slot list construction, response parsing.
- Key files: `src/proto/mod.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`.

**`tests/`:**
- Purpose: Integration and protocol-level tests.
- Contains: 3 test files and a `fixtures/` subdirectory.
- Key files: `tests/proto_tests.rs`, `tests/real_cookies.rs`, `tests/integration_tests.rs`.

**`tests/fixtures/`:**
- Purpose: Stable input data for unit and integration tests.
- Contains: captured HTML snippets, JSON responses, error wrappers, model lists.
- Key files: `tests/fixtures/wiz_global_data.txt`, `tests/fixtures/turn1_response_raw.txt`, `tests/fixtures/model_list_response.txt`.

**`examples/`:**
- Purpose: Runnable demonstrations of SDK capabilities.
- Contains: 5 example binaries declared in `Cargo.toml`.
- Key files: `examples/text_chat.rs`, `examples/image_chat.rs`, `examples/stream_chat.rs`.

**`benches/`:**
- Purpose: Performance benchmarks for hot paths.
- Contains: `benches/slot_building.rs` (criterion).

**`docs/`:**
- Purpose: Protocol documentation and design notes.
- Contains: `docs/protocol.md`.

**`.planning/`:**
- Purpose: GSD planning artifacts.
- Contains: `.planning/codebase/`, `.planning/spikes/`.
- Generated: No — maintained by agents.
- Committed: Yes.

## Key File Locations

**Entry Points:**
- `src/lib.rs`: crate root, module exports, lint config.
- `src/client.rs:80`: `GeminiClient::from_cookie_header` primary constructor.
- `examples/text_chat.rs`: simplest runnable example.

**Configuration:**
- `Cargo.toml`: dependencies, features, examples, benchmarks, MSRV.
- `rustfmt.toml`: formatting rules.
- `clippy.toml`: clippy behavior.

**Core Logic:**
- `src/client.rs`: public client, request orchestration, session init.
- `src/proto/slots.rs`: StreamGenerate slot construction.
- `src/proto/parser.rs`: response parsing.
- `src/upload.rs`: image upload flow.
- `src/auth.rs`: authentication primitives.

**Testing:**
- `tests/proto_tests.rs`: protocol unit/integration tests.
- `tests/integration_tests.rs`: high-level integration tests.
- `tests/real_cookies.rs`: live tests requiring cookies.
- Inline `#[cfg(test)]` modules in source files (`src/auth.rs`, `src/chat.rs`, `src/session.rs`, `src/proto/mod.rs`, `src/proto/slots.rs`, `src/proto/parser.rs`, `src/models.rs`).

## Naming Conventions

**Files:**
- Lowercase with underscores matching the contained module: `client.rs`, `chat.rs`, `proto/parser.rs`.
- Examples are named after the scenario: `text_chat.rs`, `image_chat.rs`.
- Test files are named after the subsystem: `proto_tests.rs`, `integration_tests.rs`, `real_cookies.rs`.

**Directories:**
- Lowercase plural for collections: `tests/`, `examples/`, `benches/`, `docs/`.
- Nested module directory matches parent module name: `src/proto/`.

## Where to Add New Code

**New public API method:**
- Implementation: `src/client.rs`.
- Builder method: `src/client.rs` (`ChatBuilder` impl).
- Tests: `tests/integration_tests.rs` or `tests/proto_tests.rs`.

**New protocol field / slot behavior:**
- Slot logic: `src/proto/slots.rs` (`build_inner_req_list`).
- Body builders: `src/proto/mod.rs`.
- Parser changes: `src/proto/parser.rs`.
- Tests: `tests/proto_tests.rs` and `src/proto/slots.rs` inline tests.

**New auth mechanism:**
- Implementation: `src/auth.rs`.
- Integration: `src/client.rs` constructors.
- Tests: `src/auth.rs` inline tests.

**New utility helper:**
- Internal-only helper: add to the relevant module or a new private module in `src/`.
- Public helper: expose via `src/lib.rs` re-exports.

**New example:**
- Implementation: `examples/<name>.rs`.
- Declaration: `Cargo.toml` `[[example]]` block.

## Special Directories

**`target/`:**
- Purpose: Cargo build artifacts.
- Generated: Yes.
- Committed: No (ignored by `.gitignore`).

**`.planning/spikes/`:**
- Purpose: Historical spike findings and captured research.
- Generated: No.
- Committed: Yes.

**`tests/fixtures/`:**
- Purpose: Stable test data.
- Generated: Partially — some are hand-written, some captured via `examples/capture_fixtures.rs`.
- Committed: Yes.

---

*Structure analysis: 2026-08-08*
