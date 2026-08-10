---
phase: 6
slug: v1-0-release
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-08-10
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Cargo test (Rust built-in) |
| **Config file** | `Cargo.toml` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test --all-features --all-targets` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo clippy --all-targets -- -D warnings && cargo doc --no-deps && cargo test --all-features --all-targets`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 6-01-01 | 01 | 1 | TOOL-05 | — | No secrets in published crate | static audit | `cargo publish --dry-run` | ✅ | ⬜ pending |
| 6-01-02 | 01 | 1 | API-04 | — | Public items documented | lint | `cargo doc --no-deps` | ✅ | ⬜ pending |
| 6-01-03 | 01 | 1 | TOOL-02 | — | No warnings | lint | `cargo clippy --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 6-02-01 | 02 | 2 | TOOL-05 | — | Accurate feature docs | manual review | `CHANGELOG.md`/`README.md` review | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Real crates.io publish | TOOL-05 | Credentials required; release control belongs to user | User runs `cargo publish` after merging |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
