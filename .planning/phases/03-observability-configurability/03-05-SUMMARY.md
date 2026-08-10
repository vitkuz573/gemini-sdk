# 03-05 Summary: Robust HTML Extraction Fallbacks (PROTO-03)

## Completed

- Added shared `try_extract_value` helper in `src/session.rs` that searches the `window.WIZ_global_data` block first, then the whole body, trying each key in order and validating the result.
- Refactored extractors to use alias fallback chains:
  - `extract_snlim0e`: `SNlM0e`, `SnlM0e`, `snlM0e`
  - `extract_build_label`: `cfb2h`, `build_label`, plus bare substring fallback
  - `extract_session_id`: `FdrFJe`, `f.sid`, `session_id`
  - `extract_push_id`: `qKIAYe`, `KnDnFf`, `push_id`
- Added validation functions (`is_valid_snlim0e`, `is_valid_build_label`, `looks_like_session_id`) shared between WIZ-block and bare-substring paths.
- Added unit tests in `src/session.rs` for:
  - Each alias key shape.
  - Fallback order (canonical key preferred over aliases).
  - Invalid values rejected by validators.
  - Existing real fixtures still pass.

## Files Modified

- `src/session.rs`

## Verification

- `cargo test --all-targets` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
