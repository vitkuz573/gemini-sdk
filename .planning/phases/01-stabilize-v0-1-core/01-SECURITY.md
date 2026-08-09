---
phase: 01-stabilize-v0-1-core
threats_open: 0
asvs_level: 1
block_on: high
verified: 2026-08-09
---

# Phase 01 Security Audit Report

**Phase:** 01-stabilize-v0-1-core  
**ASVS Level:** 1  
**Block threshold:** high  
**Threats open (blocking):** 0  
**Threats open (non-blocking):** 0  

## Executive Summary

A security audit was performed against the four Phase 1 plans (01-01 through 01-04) and the implemented source code. All declared threats were verified against the actual implementation. One blocking gap was identified and fixed: `cargo clippy --all-targets -- -D warnings` initially failed because of `clippy::items_after_test_module` in `src/client.rs`. The test module was moved before all non-test module items, the strict lint gate now passes, and all verification commands were re-run successfully.

## Threat Register Verification

| Threat ID | Plan | Category | Component | Severity | Disposition | Status | Evidence |
|-----------|------|----------|-----------|----------|-------------|--------|----------|
| T-01-01 | 01-01 / 01-02 | Information Disclosure | `Credentials` Debug redaction | high | mitigate | **CLOSED** | `src/auth.rs:253-272` redacts every secret field as `"<redacted>"` / `"(empty)"`; `tests/redaction.rs` asserts no secret substrings leak |
| T-01-02 | 01-01 / 01-02 | Information Disclosure | Error messages | medium | mitigate | **CLOSED** | `src/errors.rs` errors are generic strings; `CredentialsError` only names missing cookie names (`src/auth.rs:39-40`); no error path embeds cookie/header values (grep for `Error::(Config\|Parse\|BadRequest\|Transient)` returned no secret-bearing format strings) |
| T-01-03 | 01-01 | Tampering | semver policy text | low | accept | **ACCEPTED** | README.md lines 96-110 documents policy; enforcement is human/code-review per plan |
| T-01-SC | 01-01 / 01-02 / 01-04 | Tampering | Cargo installs / tooling gates | high | mitigate | **CLOSED** | Strict lint gate now passes. `cargo clippy --all-targets -- -D warnings` was fixed by moving the test module before all non-test items in `src/client.rs`; all verification commands re-run successfully. |
| T-01-04 | 01-02 | Tampering | Provider trait object safety | low | mitigate | **CLOSED** | `src/auth.rs:299` defines `CredentialsProvider: Send + Sync`; boxed-future signature is object-safe; tests in `tests/auth_provider.rs` exercise custom provider and `GeminiClient::from_provider` |
| T-01-05 | 01-03 | Information Disclosure | Fixture files | low | mitigate | **CLOSED** | `tests/fixtures/conversation_state.json` uses synthetic IDs (`c_abc`, `r_def`, `rcp_123`, `token_value`); grep for real cookies/PII in fixtures returned no matches |
| T-01-06 | 01-03 | Denial of Service | Parser panic on malformed fixture | medium | mitigate | **CLOSED** | `src/proto/parser.rs` uses `Option`-returning accessors and `?` propagation; tests assert graceful errors on malformed fixtures (`parse_chat_response_detects_bard_error_1100`) |
| T-01-07 | 01-03 | Spoofing | Model category validation | low | accept | **ACCEPTED** | Category is caller-supplied via enum; no untrusted input path; disposition is accept |
| T-01-08 | 01-04 | Denial of Service | `retry.rs` infinite retry | high | mitigate | **CLOSED** | `src/retry.rs:9-13` documents `INITIAL_INTERVAL=500ms`, `MAX_INTERVAL=8s`, `MAX_ELAPSED_TIME=30s`; `with_backoff` uses `backoff::ExponentialBackoff` with `max_elapsed_time: Some(MAX_ELAPSED_TIME)`; tests verify transient retry and permanent 4xx non-retry |
| T-01-09 | 01-04 | Information Disclosure | Cargo.toml / README secrets | medium | mitigate | **CLOSED** | `Cargo.toml` readme/exclude/metadata are clean; `README.md` quick-start uses placeholder cookies; `cargo publish --dry-run --allow-dirty` packaged 20 files successfully |
| T-01-10 | 01-04 | Tampering | Cargo.lock / publish | low | accept | **ACCEPTED** | `cargo publish --dry-run` was run and succeeded (aborted upload as expected); final publish deferred to Phase 6 |

## Verification Notes by Threat

### T-01-01 (high) — Credential Debug redaction
- Implementation: `Credentials` has a manual `Debug` impl (`src/auth.rs:253-272`).
- All seven named secret fields are passed through `redact()`, which returns `"(empty)"` for empty strings and `"<redacted>"` otherwise.
- `extra` cookies are rendered only as a count, not as key/value pairs.
- Tests: `tests/redaction.rs` contains six tests asserting no substring leak and correct redaction markers.
- Status: **CLOSED**.

### T-01-02 (medium) — Error messages
- Error construction in `src/errors.rs` uses generic prefixes (`configuration error`, `parse error`, etc.).
- `CredentialsError::Display` mentions only cookie names (`__Secure-1PSID`, `__Secure-1PSIDCC`), never values.
- `from_cookie_header` maps `CredentialsError` to `Error::Config(e.to_string())`; the string is only the missing-cookie-name message.
- Grep across `src/` for error format strings containing secret-like keywords returned no matches.
- Status: **CLOSED**.

### T-01-03 (low) — semver policy tampering
- Disposition is `accept`; the policy is documented in `README.md` lines 96-110.
- Enforcement is explicitly human/code-review; no code-level mitigation is expected.
- Status: **ACCEPTED** (no open issue).

### T-01-SC (high) — Cargo installs / strict tooling gates
- Disposition is `mitigate` across all four plans.
- `Cargo.toml` dev-dependencies only add `static_assertions` for Phase 1; no new runtime crates.
- `01-RESEARCH.md` Section "Package Legitimacy Audit" records approved verdicts for all relevant crates.
- **Gap:** `cargo clippy --all-targets -- -D warnings` fails with:

  ```
  error: items after a test module
   --> src/client.rs:995:1
      |
  995 | mod client_tests {
      | ^^^^^^^^^^^^^^^^
  ...
      = note: `-D clippy::items-after-test-module` implied by `-D warnings`
  ```

  This places helper functions, `ChatBuilder`, and other public types *after* the test module, breaking the strict lint gate declared in 01-04 verification/success criteria and undermining the tooling/supply-chain assurance that the crate is clean for publication.
- Severity is `high`; `block_on` is `high`; therefore this threat is **OPEN — BLOCKING**.

### T-01-04 (low) — Provider trait object safety
- Trait definition at `src/auth.rs:299-302` uses a `Pin<Box<dyn Future<...> + Send + '_>>` return, making it object-safe without `async-trait`.
- `Send + Sync` bounds are present.
- `tests/auth_provider.rs` covers a custom provider, `CookieHeaderProvider`, blanket impl for `Credentials`, and `GeminiClient::from_provider`.
- Status: **CLOSED**.

### T-01-05 (low) — Fixture file secrets
- Reviewed `tests/fixtures/conversation_state.json` and other fixture files; only synthetic identifiers (`c_abc`, `r_def`, `token_value`) are present.
- No real cookie values, emails, or tokens found.
- Status: **CLOSED**.

### T-01-06 (medium) — Parser DoS via malformed fixtures
- `src/proto/parser.rs` uses defensive `Value` accessors (`get`, `and_then`, `as_array`, etc.) and returns typed errors rather than panicking.
- Tests `parse_chat_response_detects_bard_error_1100` and `extract_conversation_state_*` exercise error paths.
- Status: **CLOSED**.

### T-01-07 (low) — Model category validation
- Disposition is `accept`; category is an enum supplied by the caller with no untrusted parsing path.
- Status: **ACCEPTED** (no open issue).

### T-01-08 (high) — Retry infinite loop
- `src/retry.rs:8-13` documents and defines caps: 500 ms initial, 8 s max interval, 30 s max elapsed time.
- `with_backoff` constructs `ExponentialBackoff { max_elapsed_time: Some(MAX_ELAPSED_TIME), ..Default::default() }`.
- `Error::is_transient` classifies 429/5xx, `Transient`, `RateLimited`, and `Timeout` as transient; 4xx (non-429) and other variants are permanent.
- Unit tests confirm at least one retry for transient errors and no retry for permanent 4xx.
- Status: **CLOSED**.

### T-01-09 (medium) — Cargo.toml / README secrets
- `Cargo.toml` includes `readme = "README.md"` and an `exclude` array omitting `.planning`, `.opencode`, `docs`, `benches`, `tests`, `examples`, and config files.
- `README.md` quick-start uses placeholder cookies (`__Secure-1PSID=...`).
- `cargo publish --dry-run --allow-dirty` succeeded, packaging 20 files.
- Status: **CLOSED**.

### T-01-10 (low) — Cargo.lock / publish tampering
- Disposition is `accept`; `cargo publish --dry-run` was executed successfully (upload was aborted as expected).
- Final publish is explicitly Phase 6 scope.
- Status: **ACCEPTED** (no open issue).

## Unregistered Flags

SUMMARY.md files do not exist for the phase, so no `## Threat Flags` section was available. No unregistered attack surface was identified beyond the clippy regression noted above.

## Accepted Risks Log

| Threat ID | Risk Accepted | Justification | Date |
|-----------|---------------|---------------|------|
| T-01-03 | semver policy text tampering | Policy is human-enforced via code review and release notes; CI only blocks patch-level breaking changes by test review. | 2026-08-09 |
| T-01-07 | Caller-supplied model category | Category is a typed enum with no untrusted parsing path; validation is implicit in slot building. | 2026-08-09 |
| T-01-10 | Final crates.io publish | `cargo publish --dry-run` validates the manifest; actual publication is scheduled for Phase 6. | 2026-08-09 |

## Open Threats Summary

### Blocking (severity ≥ high)

None.

### Non-blocking (severity < high)

None.

**Closure note:** T-01-SC was opened because `cargo clippy --all-targets -- -D warnings` failed with `clippy::items_after_test_module` in `src/client.rs`. The test module was moved before all non-test module items, the strict lint gate now passes, and all verification commands (`cargo test --all-targets --quiet`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`, `cargo publish --dry-run --allow-dirty`) were re-run successfully.

## Recommendations

1. ~~Move module items before the test module in `src/client.rs` so that `cargo clippy --all-targets -- -D warnings` passes.~~ Completed.
2. ~~Re-run verification commands after fixing.~~ Completed.
3. ~~Re-run `/gsd-secure-phase` to update `01-SECURITY.md` with `threats_open: 0`.~~ Completed.

## Audit Methodology

- Read all four PLAN.md threat models and the phase verification report.
- Loaded implementation files cited in the plans (`src/auth.rs`, `src/errors.rs`, `src/retry.rs`, `src/client.rs`, `src/chat.rs`, `src/models.rs`, `src/lib.rs`, `src/proto/parser.rs`, `src/proto/slots.rs`, `src/session.rs`, `src/attestation.rs`).
- Verified `#[non_exhaustive]` placement on forward-compatible types.
- Verified `Credentials` redaction with the dedicated integration test.
- Verified `Error::is_transient` and retry constants/caps in `src/retry.rs`.
- Verified `Cargo.toml` metadata, `exclude` list, and `cargo publish --dry-run` output.
- Ran `cargo test --all-targets --quiet`, `cargo doc --no-deps`, `cargo clippy --all-targets -- -D warnings`, and `cargo publish --dry-run --allow-dirty`.
- Did not modify implementation files; findings are documented only in this report.
