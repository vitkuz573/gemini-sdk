# Phase 6: v1.0 Release — Research

**Researched:** 2026-08-10
**Phase:** 6 — v1.0 Release
**Requirement IDs:** TOOL-05

## User Constraints (from CONTEXT.md — planner MUST honor)

### Version Policy
- Keep crate version at `0.1.0` for this milestone (v0.1 Core milestone).
- Add a CHANGELOG.md that documents all changes from initial state to v0.1.0 and notes the path toward v1.0.
- Do not bump to `1.0.0` now; the ROADMAP explicitly labels this milestone as v0.1 Core.

### crates.io Publication Readiness (TOOL-05)
- Run `cargo publish --dry-run` to verify the manifest packages correctly.
- Ensure `Cargo.toml` has all required fields: name, version, authors, edition, license, description, repository, readme, keywords, categories, rust-version.
- Ensure LICENSE file is present and correct.
- Ensure README.md is accurate and reflects current features.
- Document the exact `cargo publish` command for the user to run later.
- Do not perform a real publish; credentials are not available and release control belongs to the user.

### API Audit and Deprecation Cleanup
- Run `cargo public-api` if available; otherwise do a manual review of `src/lib.rs` re-exports.
- Remove any leftover `#[allow(unused)]` or dead code introduced during development.
- Ensure all public items have doc comments and `#![warn(missing_docs)]` passes cleanly.
- Verify no public items expose internal protocol details unintentionally.

### MSRV Verification
- Verify `rust-version = "1.80"` in Cargo.toml.
- Document MSRV policy in README.md and CONTRIBUTING.md.
- Run tests with the installed toolchain; note if a toolchain check is not possible in this environment.

### Documentation and Release Notes
- Write CHANGELOG.md with sections for each phase (v0.1.0 release notes).
- Update README.md to mention new features: hooks, tracing, injectable client, upload progress, audio/video, tools, metrics, session save/restore.
- Add a v0.1 → v1.0 migration guide section in docs/ or README.md noting breaking changes (async config builders, attestation errors).

### the agent's Discretion
- Agent may adjust changelog format and migration guide location.
- Agent may add or update examples to reflect new features.

## Standard Stack

- `cargo publish --dry-run` — crates.io packaging validator (no credentials needed) [CITED: doc.rust-lang.org/cargo/reference/publishing.html]
- `cargo doc --no-deps` — documentation build with warnings as errors
- `cargo clippy --all-targets -- -D warnings` — lint gate
- `cargo test` — unit/integration test gate
- `cargo-public-api` — public API diffing/auditing tool [VERIFIED: crates.io]
- `cargo-semver-checks` — semver linting for public API [VERIFIED: crates.io]
- Keepin `#![deny(missing_docs)]` and `#![deny(rustdoc::broken_intra_doc_links)]` already in `src/lib.rs`

## Architecture Patterns

1. **Publication-readiness checklist** — a sequential audit of manifest, license, README, docs, API surface, then a dry-run publish.
2. **API surface audit** — enumerate public items via `src/lib.rs` re-exports and compare against intended external surface; ensure no protocol internals leak.
3. **Breaking-change documentation** — capture public API changes introduced across phases in a migration guide.
4. **CHANGELOG conventions** — keepachangelog.com format (Added/Changed/Deprecated/Removed/Fixed/Security) scoped to the v0.1.0 release.
5. **MSRV policy** — document `rust-version` in Cargo.toml, CI matrix, and contributor docs.

## Don't Hand-Roll

- Do not write a custom semver analyzer when `cargo-semver-checks` exists [VERIFIED: crates.io].
- Do not hand-craft an API diff report when `cargo-public-api` exists [VERIFIED: crates.io].
- Do not invent a Cargo.toml validator — `cargo publish --dry-run` covers it.

## Common Pitfalls

- `exclude` in Cargo.toml excludes `examples/`, `tests/`, and `benches/` from the published crate, which is correct but must be documented so users know examples are in the repo, not the crate tarball.
- Running `cargo publish` (without `--dry-run`) requires crates.io credentials; the phase must only document the real command.
- `#![deny(missing_docs)]` already warns on missing docs; any new public items without docs will fail `cargo doc`.
- `cargo-public-api` and `cargo-semver-checks` are dev-dependency-like CLI tools; they can be installed on demand and should not fail the phase if unavailable in this environment, but the plan should try them and fall back to manual review.
- Public re-exports in `src/lib.rs` should not expose `proto` internals such as raw WIZ slot constants or HTML extraction helpers unless they are explicitly intended for advanced consumers.

## Code Examples

### Recommended API audit command sequence
```bash
cargo install cargo-public-api --locked 2>/dev/null || true
cargo public-api --target-version 0.1.0 2>/dev/null || echo "cargo-public-api unavailable; perform manual review"
```

### Recommended semver check
```bash
cargo install cargo-semver-checks --locked 2>/dev/null || true
cargo semver-checks --baseline-version 0.1.0 2>/dev/null || echo "No prior baseline; semver checks skipped"
```

### Cargo.toml fields required for publish
```toml
[package]
name = "gemini-sdk"
version = "0.1.0"
authors = ["Vitaly Kuzyaev <vitkuz573@gmail.com>"]
edition = "2021"
license = "MIT"
description = "..."
repository = "https://github.com/vitkuz573/gemini-sdk"
readme = "README.md"
keywords = ["gemini", "bard", "google", "ai", "sdk"]
categories = ["api-bindings", "network-programming", "asynchronous"]
rust-version = "1.80"
```

## Security Notes

- No secrets or credentials are published to crates.io; the crate only contains code and documentation.
- `Cargo.toml` `exclude` already excludes `.planning/`, `.opencode/`, and other internal files from the crate tarball, preventing accidental disclosure of planning artifacts.
- The LICENSE file must be included in the tarball; verify `license-file` is not needed because `license = "MIT"` is a known SPDX identifier.

## Verification Strategy

1. `cargo test` passes.
2. `cargo clippy --all-targets -- -D warnings` passes.
3. `cargo doc --no-deps` builds with no warnings under `#![deny(missing_docs)]`.
4. `cargo publish --dry-run` succeeds.
5. Public API review (tool-assisted or manual) confirms no unintended protocol internals are exposed.
6. CHANGELOG.md, README.md, and CONTRIBUTING.md are updated and consistent with Cargo.toml.

## Research Complete

## RESEARCH COMPLETE
