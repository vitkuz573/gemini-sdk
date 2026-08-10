---
phase: 5
slug: tools-auto-refresh
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-10
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in Rust test harness) |
| **Config file** | none — standard `cargo test` |
| **Quick run command** | `cargo test --lib <module>` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib <module>`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | ADV-01 | T-05-01 | Tool trait is object-safe and invokable | unit | `cargo test --lib tool` | ❌ W0 | ⬜ pending |
| 05-01-02 | 01 | 1 | ADV-01 | T-05-01 | Tool schemas accepted as `serde_json::Value` | unit | `cargo test --lib tool` | ❌ W0 | ⬜ pending |
| 05-02-01 | 02 | 1 | ADV-01 | T-05-03 | Tool declarations encode into `inner_req_list` | unit | `cargo test --lib proto_slots` | ❌ W0 | ⬜ pending |
| 05-02-02 | 02 | 1 | ADV-01 | T-05-03 | Slot 0 shape preserved when no tools present | unit | `cargo test --lib proto_slots` | ❌ W0 | ⬜ pending |
| 05-03-01 | 03 | 2 | ADV-01 | T-05-03 | Parser extracts `ToolCall` parts from WIZ frame | unit | `cargo test --lib parser` | ❌ W0 | ⬜ pending |
| 05-03-02 | 03 | 2 | ADV-01 | T-05-04 | Parser falls back to text on unknown tool shape | unit | `cargo test --lib parser` | ❌ W0 | ⬜ pending |
| 05-04-01 | 04 | 2 | ADV-01 | T-05-04 | `generate_with_tools` invokes tools and sends follow-up | integration | `cargo test --test integration_tests tools` | ❌ W0 | ⬜ pending |
| 05-04-02 | 04 | 2 | ADV-01 | T-05-05 | Tool-call recursion capped at configured limit | integration | `cargo test --test integration_tests tools` | ❌ W0 | ⬜ pending |
| 05-05-01 | 05 | 2 | ADV-03 | T-05-02 | `refresh_credentials` replaces cookies and re-inits session | integration | `cargo test --test auth_provider refresh` | ❌ W0 | ⬜ pending |
| 05-06-01 | 06 | 3 | ADV-03 | T-05-02 | `with_refresh_on_auth_error` retries once on `NotSignedIn` | integration | `cargo test --test integration_tests refresh_retry` | ❌ W0 | ⬜ pending |
| 05-07-01 | 07 | 3 | OBS-03 | T-05-06 | No-op recorder compiles and has zero overhead | unit | `cargo test --lib metrics` | ❌ W0 | ⬜ pending |
| 05-07-02 | 07 | 3 | OBS-03 | T-05-06 | OpenTelemetry recorder emits counter/histogram (feature) | unit | `cargo test --lib metrics --features metrics` | ❌ W0 | ⬜ pending |
| 05-08-01 | 08 | 3 | OBS-03 | T-05-06 | Request/retry/parse/attestation boundaries record metrics | integration | `cargo test --test integration_tests metrics` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/tool.rs` + unit tests in `src/tool.rs` or `tests/tool.rs` — covers ADV-01
- [ ] `src/metrics.rs` + unit tests in `src/metrics.rs` or `tests/metrics.rs` — covers OBS-03
- [ ] Extend `src/proto/parser.rs` tests for tool-call parts — covers ADV-01
- [ ] Extend `src/proto/slots.rs` tests for tool metadata — covers ADV-01
- [ ] Extend `tests/integration_tests.rs` for tool round-trip and refresh retry — covers ADV-01, ADV-03
- [ ] Extend `tests/auth_provider.rs` for explicit refresh — covers ADV-03
- [ ] Add `metrics` feature to `Cargo.toml` — covers OBS-03

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Tool round-trip against live Gemini frontend | ADV-01 | Undocumented protocol shape requires real HAR fixture | Capture a live `StreamGenerate` request/response with a registered tool; update snapshot fixture in `tests/fixtures/` |
| Consent re-acquisition after cookie refresh | ADV-03 | Requires real Google account and browser cookies | Refresh cookies from an account that shows a consent banner; verify `SOCS` cookie is merged |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
