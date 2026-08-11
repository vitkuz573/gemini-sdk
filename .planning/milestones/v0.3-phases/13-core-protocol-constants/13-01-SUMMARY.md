# Phase 13: Core Protocol Constants — Summary

**Phase:** 13-core-protocol-constants  
**Plan:** 01  
**Status:** Complete  
**Completed:** 2026-08-11

## What Changed

- Created `src/constants.rs` with centralized, documented constants for:
  - Base URLs and URL paths (`urls`)
  - Batchexecute query keys (`query_keys`)
  - Transport markers including `ANTI_XSSI_PREFIX` (`transport`)
  - WIZ/session extraction keys (`wiz_keys`)
  - RPC identifiers (`rpc_ids`) including `otAQ7b` and `Fd0Qje`
- Re-exported `ANTI_XSSI_PREFIX` and `RPC_FRAME_MARKER` from `src/proto/mod.rs` and `src/proto/indices.rs` respectively.
- Refactored `src/client.rs` to use constants for batchexecute URLs, source paths, query keys, and endpoint discriminator.
- Refactored `src/session.rs` to use WIZ-key constants for extraction and `otAQ7b` for reqid base selection.

## Verification

- `cargo test --all-targets` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo doc --no-deps` passed.

## Notes

- Public API signatures unchanged.
- Constants remain `pub(crate)` except where existing re-exports required `pub`.
- Remaining inline literals in tests and parser/har/transient_400 fixtures are intentional (tests assert behavior; parser compares captured protocol values).
