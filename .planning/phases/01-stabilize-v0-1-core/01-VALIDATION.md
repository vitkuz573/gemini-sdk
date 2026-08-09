---
phase: 01
slug: stabilize-v0-1-core
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-08-09
---

# Phase 01 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) + `tokio-test` for async tests |
| **Config file** | `Cargo.toml` (dev-dependencies and features) |
| **Quick run command** | `cargo test --lib --quiet` |
| **Full suite command** | `cargo test --all-targets` |
| **Estimated runtime** | ~15–30 seconds (fixture-based, no live cookies) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib --quiet`
- **After every plan wave:** Run `cargo test --all-targets`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | API-01 | — | `#[non_exhaustive]` on public types | unit/integration | `cargo test --test api_stability` | ✅ | ⬜ pending |
| 01-01-02 | 01 | 1 | API-02 | — | `Error: Send + Sync + 'static` | unit | `cargo test --lib error_traits` | ✅ | ⬜ pending |
| 01-01-03 | 01 | 1 | API-04 | — | `#![deny(missing_docs)]` passes | compile | `cargo doc --no-deps` | ✅ | ⬜ pending |
| 01-02-01 | 02 | 2 | AUTH-01 | T-01-02 | Cookie header parsing validates required cookies | unit | `cargo test --lib auth::cookies` | ✅ | ⬜ pending |
| 01-02-02 | 02 | 2 | AUTH-02 | — | `CredentialsProvider` trait has default impl | integration | `cargo test --test auth_provider` | ✅ | ⬜ pending |
| 01-02-03 | 02 | 2 | AUTH-03 | T-01-01 | Debug output redacts secrets | integration | `cargo test --test redaction` | ✅ | ⬜ pending |
| 01-03-01 | 03 | 2 | CHAT-01/CHAT-03/CHAT-05 | — | Text chat + multi-turn + model category via fixtures | integration | `cargo test --test integration_tests` | ✅ | ⬜ pending |
| 01-03-02 | 03 | 2 | MEDIA-01 | — | Inline image upload produces upload ID | integration | `cargo test --test proto_tests` | ✅ | ⬜ pending |
| 01-04-01 | 04 | 3 | REL-01 | — | Transient errors trigger exponential backoff | unit | `cargo test --lib retry` | ✅ | ⬜ pending |
| 01-04-02 | 04 | 3 | TOOL-01/TOOL-02/TOOL-03/TOOL-04 | — | Full suite, clippy, docs, examples green | compile + integration | `cargo test --all-targets && cargo clippy --all-targets -- -D warnings && cargo doc --no-deps` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/auth_provider.rs` — tests for `CredentialsProvider` and `CookieHeaderProvider` (created by Plan 01-02)
- [x] `tests/api_stability.rs` — compile-time checks for `#[non_exhaustive]` and error trait bounds (created by Plan 01-04 Wave 0)
- [x] `tests/redaction.rs` — Debug output does not leak cookies (created by Plan 01-04 Wave 0)

*All Wave 0 test files are referenced in the phase plans.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `cargo publish --dry-run` succeeds | TOOL-05 (pre-flight) | Requires crates.io credentials and clean workspace | Run `cargo publish --dry-run` locally before Phase 6 final publication |
| Live cookie integration tests pass | CHAT-01, CHAT-02, MEDIA-01 | Requires real `GEMINI_COOKIES` env var; cannot run in CI | Export `GEMINI_COOKIES` and run `cargo test --test real_cookies -- --ignored` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
