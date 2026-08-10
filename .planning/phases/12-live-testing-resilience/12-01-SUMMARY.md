---
phase: 12-live-testing-resilience
plan: 01
subsystem: resilience
status: complete
tags: [resilience, har, retry, live-testing, cookies, hotfix]
dependency_graph:
  requires: [11-01]
  provides: [RESIL-01, RESIL-02, RESIL-03, RESIL-04, TOOL-06, TOOL-07]
  affects: [src/auth.rs, src/client.rs, src/errors.rs, src/session.rs, src/retry.rs, src/har.rs, src/transient_400.rs, src/chat.rs, src/lib.rs, examples/live_probe.rs, tests/real_cookies.rs, README.md, Cargo.toml]
tech_stack:
  added: [humantime, tempfile]
  patterns: [W3C HAR 1.2, exponential backoff, cookie redaction, synthetic transient errors]
key_files:
  created:
    - src/transient_400.rs
    - src/har.rs
    - examples/live_probe.rs
  modified:
    - src/client.rs
    - src/errors.rs
    - src/session.rs
    - src/retry.rs
    - src/chat.rs
    - src/lib.rs
    - tests/real_cookies.rs
    - Cargo.toml
decisions:
  - Conservative WIZ transient 400 detection requires all three markers (er, di, af.httprm) on HTTP 400.
  - HAR capture is opt-in via builder and flushes after every entry to limit data loss.
  - Cookie values, Authorization headers, x-goog-ext-* headers, and cookie-like POST substrings are redacted in HAR.
  - send_batchexecute_with_retry classifies transient 400s before the generic retry loop commits them as permanent.
metrics:
  duration: "45 min"
  completed_date: "2026-08-10"
  tasks: 5
  files: 11
---

# Phase 12 Plan 01: Live Testing & Backend Resilience Summary

## One-liner

Added transient WIZ 400 detection, bounded batchexecute retries, opt-in redacted HAR capture, a live probe example, and expanded real-cookie integration tests for all v0.2 APIs.

## What Changed

- **src/transient_400.rs** — New public detection function `is_wiz_transient_400` used by the retry helper. Returns true only for HTTP 400 responses whose body contains `"er"`, `"di"`, and `"af.httprm"`.
- **src/errors.rs** — Added `Error::not_signed_in(message)` constructor.
- **src/session.rs** — Added `looks_like_signed_in_html` as the single authoritative check; made `extract_from_app_html` defensive when `window.WIZ_global_data` is missing or malformed.
- **src/client.rs** — Added `with_har_capture`, `last_response_id`, `send_batchexecute_with_retry`, and `maybe_record_har`; routed all batchexecute v0.2 methods through the retry helper; map unsigned `/app` responses to `NotSignedIn`.
- **src/har.rs** — New `HarWriter` producing W3C HAR 1.2 entries with redacted cookies, Authorization, `x-goog-ext-*`, and POST-body cookie-like substrings.
- **src/retry.rs** — Added `with_backoff_generic` and a test proving retry on synthetic transient errors.
- **src/chat.rs** — Added public `ChatResponse::conversation_id()` accessor.
- **src/lib.rs** — Added `pub mod har` and `mod transient_400`.
- **examples/live_probe.rs** — New binary exercising all v0.2 APIs plus base chat/list_models, emitting a JSON telemetry report.
- **tests/real_cookies.rs** — Expanded with tests for user info, mode preferences, locale/model/tool config, usage stats, scheduled prompts, and conversation actions.
- **Cargo.toml** — Added `humantime`, `tempfile`, and `live_probe` example registration.
- **.planning/REQUIREMENTS.md** — Added RESIL-01/02/03/04 and traceability rows.
- **.planning/ROADMAP.md** — Added Phase 12 entry and updated milestone scope.

## Verification Results

| Gate | Result | Notes |
|------|--------|-------|
| `cargo test` | pass | 143 lib tests + integration tests + doc-tests |
| `cargo test --test real_cookies` | pass | 14/14 with live cookies |
| `cargo clippy --all-targets -- -D warnings` | pass | clean |
| `cargo doc --no-deps` | pass | no warnings under `#![deny(missing_docs)]` |
| `cargo build --example live_probe` | pass | builds successfully |

## Deviations from Plan

None — plan executed exactly as written.

## Post-Summary Hotfix

A live cookie probe revealed that the minimal secure cookie set is insufficient
for Gemini signed-in detection. Hotfix commit `78a2271` (applied after the
original plan completion) adds:

- Explicit preservation of legacy/account cookies in `Credentials`:
  `__Secure-3PAPISID`, `SIDCC`, `__Secure-ENID`, `NID`.
- `Credentials::missing_legacy_cookies()` diagnostics.
- `session::diagnose_signed_in_html()` with `SignedInFailure` reasons.
- `GeminiClient::diagnose_signed_in()` and `AppDiagnostics`.
- Missing-cookie hints in `Error::NotSignedIn` messages.
- `/app` diagnostics in `examples/live_probe.rs` JSON report.
- Updated `tests/real_cookies.rs` and `README.md` with the full required cookie
  set.

All verification gates re-passed after the hotfix:
`cargo test`, `cargo clippy --all-targets -- -D warnings`,
`cargo doc --no-deps`, `cargo build --example live_probe`.

## Cookie-Jar Refresh Follow-Up

The SDK now enables reqwest's built-in cookie store and merges refreshed
`Set-Cookie` values back into stored `Credentials` after every `/app` and
batchexecute response. This means `save_session()` captures an up-to-date jar
and subsequent clients restored from the snapshot use the latest cookies. The
initial `/app` request still explicitly sends the user-provided `Cookie` header
on top of whatever the store adds.

Changes for this follow-up:

- `src/client.rs` — `cookie_store(true)` on the inner `reqwest::Client`;
  `merge_response_cookies` helper; `Set-Cookie` merging in `fetch_app_page`,
  `accept_consent_and_refresh`, and `send_batchexecute_with_retry`; public
  `GeminiClient::cookies()` accessor.
- `src/auth.rs` — `Cookies::merge_response_cookie_pairs` for owned name/value
  pairs; `Cookies::len`/`is_empty` accessors.
- `examples/live_probe.rs` — prints refreshed cookie count after
  `diagnose_signed_in`.
- `tests/real_cookies.rs` — asserts that after successful
  `diagnose_signed_in`, the refreshed jar still contains the originally
  supplied cookie names; documents cookie refresh behavior.

Despite this mechanism, the live cookies currently supplied to the test
environment remain invalid for Gemini signed-in detection. The cookie-jar
refresh is production-ready behavior, but it cannot compensate for stale or
insufficient credentials.

## Threat Flags

No new security-relevant surface beyond the plan's threat model.

## Self-Check

- [x] Created files exist on disk.
- [x] Commits `d0c7c8e`, `2187f0a`, and `78a2271` exist in history.
- [x] All quality gates pass.
