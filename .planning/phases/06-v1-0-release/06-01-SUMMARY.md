---
phase: 06-v1-0-release
plan: 01
subsystem: release
status: complete
completed: 2026-08-10
requirements: [TOOL-05, API-04, TOOL-02, TOOL-03, TOOL-01]
---

# Phase 6 Plan 01 Summary: API Audit, Deprecation Cleanup, and Tooling Gates

## Objective

Run the final pre-publish tooling gates and public-API audit for the v0.1.0
release, delivering TOOL-05 readiness.

## What Changed

- **Cargo.toml audit**: Confirmed all required publish fields are present
  (name, version, authors, edition, license, description, repository, readme,
  keywords, categories, rust-version). Version remains `0.1.0`; license matches
  `LICENSE`.
- **Public API audit (`src/lib.rs`)**: Reviewed re-exports and module visibility.
  - `pub mod proto` remains public because downstream benches/tests use it, but
    it does not leak raw WIZ slot internals in the crate root.
  - `session` and `retry` remain private modules.
  - `Snapshot`, `PreparedRequest`, and response parsing helpers are intentionally
    public.
- **Dead-code / lint cleanup**: Removed `clippy::too_many_lines` from the
  crate-level allow list and added explanatory `// REASON:` comments to the two
  remaining `#[allow(clippy::too_many_arguments)]` suppressions.
- **Documentation**: Added clarifying doc comment on `PreparedRequest` noting it
  is exposed for benchmarks/hooks/advanced use and is not covered by primary
  semver guarantees.

## Verification

```bash
cargo test --all-features --all-targets          # passed
cargo clippy --all-targets --all-features -- -D warnings  # passed
cargo doc --no-deps --all-features                 # passed, 0 warnings
cargo publish --dry-run --all-features --allow-dirty      # passed
```

## Notes

- `cargo-public-api` could not run because `rustup` is not installed in this
  environment; a manual review of `src/lib.rs` re-exports was performed instead.
- The `cargo publish --dry-run --all-features` warnings about excluded examples,
  tests, and benches are expected because those files are intentionally excluded
  by `Cargo.toml`.
