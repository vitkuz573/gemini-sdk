# Phase 14: Model, Chat & Upload Constants - Context

**Gathered:** 2026-08-11
**Status:** Ready for planning
**Mode:** Auto-generated (infrastructure phase — smart discuss skipped per autonomy rules)

<domain>
## Phase Boundary

Centralize model/category strings, chat roles, MIME types, and upload headers/endpoints as named constants. Refactor `src/models.rs`, `src/chat.rs`, `src/upload.rs`, and `src/client.rs` to consume these constants without changing public API behavior.

</domain>

<decisions>
## Implementation Decisions

### the agent's Discretion
All implementation choices are at the agent's discretion — pure infrastructure phase. Use the existing PLAN.md, codebase conventions, and success criteria to guide decisions.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/constants.rs` created in Phase 13 for cross-cutting protocol constants.
- Existing model/category strings in `src/models.rs`.
- Existing chat role and content-type strings in `src/chat.rs`.
- Existing upload endpoint and header strings in `src/upload.rs`.

### Established Patterns
- Constants are `pub(crate)` unless already public.
- Co-locate feature-specific constants in their module when they are not cross-cutting.
- Quality gate: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo doc --no-deps`.

### Integration Points
- `src/models.rs` exposes public types but internal strings can be crate-private constants.
- `src/chat.rs` builder and request preparation use role strings and content-type markers.
- `src/upload.rs` uses upload URLs and headers.
- `src/client.rs` references model categories and upload flow.

</code_context>

<specifics>
## Specific Ideas

No specific requirements — refer to `14-01-PLAN.md` for the detailed task list and success criteria.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
