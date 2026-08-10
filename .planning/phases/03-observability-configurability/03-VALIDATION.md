---
phase: 03
slug: observability-configurability
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-10
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) + `tokio-test` for async |
| **Config file** | none — standard Cargo layout |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test --all-targets` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test --all-targets`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | OBS-01 | T-03-01 / — | Hook errors do not leak secrets | unit | `cargo test --lib hooks` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 1 | OBS-01 | T-03-02 / — | Hook called on request and response | unit | `cargo test --lib hooks` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 1 | OBS-02 | T-03-03 / — | Spans exclude credentials and prompt content | unit | `cargo test --lib tracing` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 1 | OBS-02 | T-03-04 / — | Public methods create expected spans | unit | `cargo test --lib tracing` | ❌ W0 | ⬜ pending |
| 03-03-01 | 03 | 2 | REL-04 | T-03-05 / — | Injected client used without rebuild | unit | `cargo test --lib from_http_client` | ❌ W0 | ⬜ pending |
| 03-04-01 | 04 | 2 | MEDIA-02 | T-03-06 / — | Progress stream yields Progress then Complete | unit | `cargo test --lib upload_progress` | ❌ W0 | ⬜ pending |
| 03-05-01 | 05 | 2 | PROTO-03 | T-03-07 / — | Extractors fall back through alias keys | unit | `cargo test --lib session_extractors` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/unit/hooks.rs` — stubs for OBS-01
- [ ] `tests/unit/tracing.rs` — stubs for OBS-02
- [ ] `tests/unit/http_client.rs` — stubs for REL-04
- [ ] `tests/unit/upload_progress.rs` — stubs for MEDIA-02
- [ ] `tests/unit/session_extractors.rs` — stubs for PROTO-03
- [ ] Shared fixtures for each HTML alias shape under `tests/fixtures/`

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Hook output observable in a real tracing subscriber | OBS-01 | Requires end-to-end subscriber setup | Run `examples/text_chat.rs` with `RUST_LOG=gemini_sdk=debug` and verify hook logs appear. |
| Upload progress increments against real upload endpoint | MEDIA-02 | Requires live cookies and network | Run `examples/image_chat.rs` with progress callback and watch byte counts increase. |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
