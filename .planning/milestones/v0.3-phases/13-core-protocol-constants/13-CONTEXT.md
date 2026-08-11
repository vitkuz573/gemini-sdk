# Phase 13: Core Protocol Constants - Context

**Gathered:** 2026-08-11
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — smart discuss skipped per autonomy rules)

<domain>
## Phase Boundary

Create a cross-cutting `src/constants.rs` module that centralizes protocol literals (URL paths, batchexecute query keys, transport markers, WIZ/session keys) and complete the RPC-ID constant set. Refactor `src/client.rs`, `src/proto/mod.rs`, `src/session.rs`, and `src/lib.rs` to consume these constants without changing public API behavior.

</domain>

<decisions>
## Implementation Decisions

### the agent's Discretion
All implementation choices are at the agent's discretion — pure infrastructure phase. Use the existing PLAN.md, codebase conventions, and success criteria to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- Existing `pub(crate) const` RPC IDs in `conversation_actions.rs`, `user_profile.rs`, `locale_model_config.rs`, `settings.rs`.
- `ANTI_XSSI_PREFIX` currently lives in `src/proto/mod.rs`.
- `src/proto/indices.rs` defines `pub const RPC_ID: &str = "wrb.fr";`.

### Established Patterns
- Constants are `pub(crate)` unless already public.
- Module re-exports in `src/lib.rs` are explicit.
- Doc comments are required for all public items (`#![warn(missing_docs)]`).
- Quality gate: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`.

### Integration Points
- `src/client.rs` builds URLs and query strings.
- `src/proto/mod.rs` builds batchexecute bodies and references RPC IDs.
- `src/session.rs` extracts WIZ keys from `window.WIZ_global_data`.
- `src/proto/indices.rs` references the `wrb.fr` frame marker.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — refer to `13-01-PLAN.md` for the detailed task list and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
