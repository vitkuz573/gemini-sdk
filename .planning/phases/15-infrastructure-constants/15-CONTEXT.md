# Phase 15: Infrastructure Constants - Context

**Gathered:** 2026-08-11
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — smart discuss skipped per autonomy rules)

<domain>
## Phase Boundary

Centralize headers/base URLs, HAR/redaction strings, transient markers, tracing/metrics, CDP/attestation, and tool schema keys as named constants. Refactor `src/client.rs`, `src/har.rs`, `src/transient_400.rs`, `src/metrics.rs`, `src/attestation.rs`, and `src/tool.rs` to consume these constants without changing public API behavior.

</domain>

<decisions>
## Implementation Decisions

### the agent's Discretion
All implementation choices are at the agent's discretion — pure infrastructure phase. Use the existing PLAN.md, codebase conventions, and success criteria to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/constants.rs` already contains `urls`, `query_keys`, `transport`, `wiz_keys`, `rpc_ids`, `mime`, `roles`, `model_keywords`, and `upload` modules.
- `src/client.rs` contains header names, user agent, base URLs, API keys, and cookie names.
- `src/har.rs` contains redaction strings and HAR keys.
- `src/transient_400.rs` contains WIZ transient markers.
- `src/metrics.rs` contains tracing operation names and metric names.
- `src/attestation.rs` contains CDP/attestation strings.
- `src/tool.rs` contains JSON schema keys.

### Established Patterns
- Constants are `pub(crate)` unless already public.
- Feature-specific constants are co-located when not cross-cutting.
- Quality gate: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`.

### Integration Points
- `src/client.rs` uses header constants for every request.
- `src/har.rs` uses redaction patterns for sensitive data.
- `src/transient_400.rs` uses marker strings for 400 detection.
- `src/metrics.rs` emits operation names and counters.
- `src/attestation.rs` drives headless Chrome CDP.
- `src/tool.rs` builds function-call schemas.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — refer to `15-01-PLAN.md` for the detailed task list and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
