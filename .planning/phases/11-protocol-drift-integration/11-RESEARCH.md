---
phase: 11
name: Protocol Drift & Integration
milestone: v0.2 API Expansion
created: 2026-08-10
---

# Phase 11 Research: Protocol Drift & Integration

## Source of Drift

Spike 001 (`~/mitm.har`, 135 MB, 863 entries) observed the following header difference:

| Header | HAR Capture | SDK Current |
|--------|-------------|-------------|
| `x-client-data` | `CNeOywE=` | `CI7yygE=` |

The SDK constant `X_CLIENT_DATA` in `src/client.rs` line 106 is the single source of truth and is referenced in:

1. `GeminiClient::build_headers` — applied to every `batchexecute` and `StreamGenerate` request.
2. `GeminiClient::waa_create` — applied to WAA Create calls.
3. `GeminiClient::ogads_get_async_data` — applied to ogads GetAsyncData calls.

Updating the constant value propagates to all three call sites automatically.

## New v0.2 RPCs Introduced in Phases 7-10

| RPC | Method(s) | Phase | Requirement | Fixture File(s) | Integration Test |
|-----|-----------|-------|-------------|-----------------|------------------|
| `PCck7e` | `regenerate_turn`, `rate_turn`, `delete_turn` | Phase 7 | CONVACT-01..04 | `tests/fixtures/pcck7e_success.txt`, `pcck7e_error.txt` | `regenerate_turn_sends_pcck7e_payload`, `rate_turn_sends_rating_value`, `delete_turn_reports_failure_on_error_payload` |
| `o30O0e` | `get_user_info` | Phase 8 | USER-01..02 | `tests/fixtures/o30O0e_user_info.txt`, `o30O0e_user_info_partial.txt` | `get_user_info_parses_full_profile`, `get_user_info_tolerates_missing_and_null_fields` |
| `L5adhe` | `get_last_selected_mode`, `set_last_selected_mode` | Phase 8 | PREFS-01..03 | `tests/fixtures/L5adhe_last_mode.txt`, `L5adhe_null_mode.txt` | `get_last_selected_mode_returns_mode_id`, `get_last_selected_mode_returns_none_for_null`, `set_last_selected_mode_sends_l5adhe_payload` |
| `cYRIkd` | `get_locale_tools` | Phase 9 | LOCALE-01,05 | `tests/fixtures/cYRIkd_locale_tools.txt` | `get_locale_tools_returns_value` |
| `whPPme` | `get_model_config` | Phase 9 | LOCALE-02,05 | `tests/fixtures/whPPme_model_config.txt` | `get_model_config_returns_value` |
| `Te6DCf` | `get_locale_config` | Phase 9 | LOCALE-03,05 | `tests/fixtures/Te6DCf_locale_config.txt` | `get_locale_config_returns_value` |
| `ku4Jyf` | `get_tools_config` | Phase 9 | LOCALE-04,05 | `tests/fixtures/ku4Jyf_tools_config.txt` | `get_tools_config_returns_value` |
| `jSf9Qc` | `get_usage_stats` | Phase 10 | SETTINGS-01,03 | `tests/fixtures/jSf9Qc_usage_stats.txt` | `get_usage_stats_returns_value` |
| `XPSWpd` | `get_scheduled_prompts` | Phase 10 | SETTINGS-02,03 | `tests/fixtures/XPSWpd_scheduled_prompts.txt` | `get_scheduled_prompts_returns_value` |

### Fixture Test Audit (TOOL-06)

All nine v0.2 RPCs have dedicated mocked fixture integration tests using `wiremock` and synthetic captured/synthetic response fixtures. No additional RPCs are required for the v0.2 milestone.

### Gaps

None. The fixture coverage is complete for the v0.2 scope per REQUIREMENTS.md.

## Example Design

A new example `examples/v0_2_api_tour.rs` will demonstrate read-only v0.2 APIs:

1. Construct a `GeminiClient` from `GEMINI_COOKIES`.
2. Call `client.get_user_info().await` and print the user's name/email.
3. Call `client.get_last_selected_mode().await` and print the mode id.
4. Call `client.get_locale_tools().await` and print the inner JSON value.
5. Call `client.get_model_config().await` and print the inner JSON value.
6. Call `client.get_usage_stats().await` and print the inner JSON value.

The example will use `tracing_subscriber::fmt::init()` for observability and will return `gemini_sdk::Result<()>` so it compiles cleanly without unhandled results. It is intentionally read-only (no mutations or conversation actions) to be safe for users to run against live cookies.

## Quality Gates

Final verification commands:

```bash
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
cargo build --examples
```

Expected outcome: all tests pass (two `#[ignore]` live-cookie tests remain ignored), clippy is clean, docs build without warnings, and the new example compiles.
