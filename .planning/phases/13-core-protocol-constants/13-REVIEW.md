---
phase: 13-core-protocol-constants
reviewed: 2026-08-11T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - src/constants.rs
  - src/lib.rs
  - src/proto/mod.rs
  - src/proto/indices.rs
  - src/session.rs
  - src/client.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 13: Code Review Report

**Reviewed:** 2026-08-11
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Phase 13 introduces `src/constants.rs` as a centralized home for protocol literals and wires it into `client.rs`, `session.rs`, and the `proto` modules. The refactor is mechanically sound: it reduces magic-string duplication, keeps the public API unchanged, and passes the full test suite, clippy, and doc generation. No ship-blocking correctness or security defects were introduced by the constant migration itself. The remaining findings are warnings about consistency and a few style nits.

## Structural Findings (fallow)

No structural findings were provided.

## Narrative Findings (AI reviewer)

### Warnings

#### WR-01: `GET_USER_STATUS_RPC_ID` is unused and its name conflicts with the RPC it names

**File:** `src/constants.rs:86`
**Issue:** The constant is named `GET_USER_STATUS_RPC_ID` but holds the value `Fd0Qje`, which is the actual `GetUserInfo` / signed-in diagnostics RPC id (see `client.rs:800-801`). It is also not referenced anywhere in the codebase after its introduction. This creates two risks: (1) a future author may import it thinking it is the right name for the value, and (2) the misleading name makes the constants module harder to trust.
**Fix:** Either remove `GET_USER_STATUS_RPC_ID` until a later phase wires up `Fd0Qje`, or rename it to `FD0QJE_RPC_ID` / `GET_USER_INFO_RPC_ID` to match the doc comment and actual usage. If kept, add a `#[allow(dead_code)]` annotation is unnecessary once it is wired in.

#### WR-02: `batchexecute_rpc` still hard-codes `/app` and `/` fallback literals

**File:** `src/client.rs:2347`
**Issue:** The `source_path_override.unwrap_or("/app")` literal is not derived from `constants::urls::APP_PATH`. Similarly, warm-up calls in `run_waa_init_chain` pass `Some("/")` and `None` rather than `DEFAULT_SOURCE_PATH`. The phase's goal was to centralize these paths, so leaving the low-level helper with string literals partially defeats the purpose.
**Fix:** Change `source_path_override.unwrap_or("/app")` to `source_path_override.unwrap_or(crate::constants::urls::APP_PATH)` and replace the `Some("/")` warm-up calls with `Some(crate::constants::urls::DEFAULT_SOURCE_PATH)`.

#### WR-03: `build_batchexecute_body_for_rpc` comment lost a closing bracket

**File:** `src/proto/mod.rs:68`
**Issue:** The comment now reads `// [[[rpcid, inner, null, "generic"]].` — it is missing one closing `]` and one closing `)`. The original comment was `[[[rpcid, inner, null, "generic"]]].`. While this is purely documentation, it is confusing for readers trying to verify the exact JSON shape.
**Fix:** Restore the correct comment: `[[[rpcid, inner, null, "generic"]]].`.

### Info

#### IN-01: `lib.rs` module and re-export ordering changed unnecessarily

**File:** `src/lib.rs:67-75` and `src/lib.rs:91-111`
**Issue:** `cargo fmt` reordered `pub mod` declarations and `pub use` groups alphabetically. This is harmless and consistent, but it enlarges the diff and makes the phase's actual change (adding `pub mod constants`) harder to review in isolation.
**Fix:** No functional change required. Consider separating pure formatting commits from semantic commits in future phases to keep review diffs focused.

#### IN-02: `ANTI_XSSI_PREFIX` and `RPC_FRAME_MARKER` are now `pub` but not documented as public API

**File:** `src/constants.rs:56` and `src/constants.rs:62`
**Issue:** These constants were previously `pub const` in `proto/mod.rs` and `proto/indices.rs`, so re-exporting them at `pub` visibility preserves the existing API. However, `constants.rs` marks most other items `pub(crate)`. There is no rationale in the module docs for why these two specific values remain public while `F_REQ_KEY` and `BATCHEXECUTE_ENDPOINT` are crate-private.
**Fix:** Add a brief note in the `transport` module doc explaining that `ANTI_XSSI_PREFIX` and `RPC_FRAME_MARKER` are public because they are re-exported by `proto` modules for downstream parsing helpers.

---

_Reviewed: 2026-08-11_
_Reviewer: gsd-code-reviewer (OpenCode)_
_Depth: standard_
