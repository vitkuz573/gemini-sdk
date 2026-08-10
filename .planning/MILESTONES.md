# Milestones

## v0.2 API Expansion (Shipped: 2026-08-10)

**Phases completed:** 11 phases, 24 plans, 22 tasks

**Key accomplishments:**

- Public API surface locked with #[non_exhaustive] types, deny-level doc lints, compile-time Error trait checks, and a documented semver policy.
- Introduced a pluggable `CredentialsProvider` trait and fully redacted credential `Debug` output without adding runtime dependencies.
- Fixture-driven tests for text chat, multi-turn state, model category slots, and inline image encoding, plus a multi-turn example binary
- Locked retry/backoff behavior, fixed clippy/doc gates, and made the crate publishable with a reviewed manifest.
- 02-reliability-protocol-hardening
- 05-tools-auto-refresh
- 05-tools-auto-refresh
- 05-tools-auto-refresh
- Typed conversation-action methods (`regenerate_turn`, `rate_turn`, `delete_turn`) on `GeminiClient` using `PCck7e`, backed by configurable `base_url` and wiremock fixture tests.
- Typed public APIs for signed-in user identity (`o30O0e`) and last-selected mode preference (`L5adhe`) backed by wiremock fixtures.
- Added `get_usage_stats` and `get_scheduled_prompts` as thin typed batchexecute RPC facades with Wiremock fixtures and opaque `serde_json::Value` wrappers.

---

## v0.1 v0.1 Core (Shipped: 2026-08-10)

**Phases completed:** 6 phases, 19 plans, 12 tasks

**Key accomplishments:**

- Public API surface locked with #[non_exhaustive] types, deny-level doc lints, compile-time Error trait checks, and a documented semver policy.
- Introduced a pluggable `CredentialsProvider` trait and fully redacted credential `Debug` output without adding runtime dependencies.
- Fixture-driven tests for text chat, multi-turn state, model category slots, and inline image encoding, plus a multi-turn example binary
- Locked retry/backoff behavior, fixed clippy/doc gates, and made the crate publishable with a reviewed manifest.
- 02-reliability-protocol-hardening
- 05-tools-auto-refresh
- 05-tools-auto-refresh
- 05-tools-auto-refresh

---
