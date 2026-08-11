---
status: resolved
trigger: "Debug why the Gemini SDK list_models fails with a WIZ 400 anti-bot response while a standalone reqwest probe with the same parameters succeeds."
created: "2026-08-11T00:00:00Z"
updated: "2026-08-11T12:00:00Z"
resolution_commit: "cb9cc02"
---

## Resolution Summary

root_cause_confirmed: "commit 2e4b392 changed run_waa_init_chain to surface failures, but it made the non-WAA batchexecute warm-up steps (otAQ7b, sJBwce, ESY5D) fatal as well. The sJBwce step returned a WIZ 400 for this cookie set, which aborted session init before any public API method could run. The public list_models error was the propagated sJBwce failure, not a list_models-specific anti-bot rejection. A nested session lock in list_models would also deadlock once init succeeded."
fix_applied: "Commit cb9cc02 tolerated failures from the batchexecute warm-up RPCs (otAQ7b, sJBwce, ESY5D) inside run_waa_init_chain, fell back to a synthetic WAA context when ogads GetAsyncData failed, kept WAA Create failures as AttestationFailed, and reused the existing session guard in list_models to avoid a nested lock."
verification_result: "probe_external and live_probe now pass list_models; full cargo test suite passes; clippy clean."
resolved_at: "2026-08-11T12:00:00Z"

## Symptoms

expected: "SDK list_models should return the model list (HTTP 200) like the standalone reqwest probe does"
actual: "SDK list_models returns HTTP 400 Bad Request with WIZ anti-bot payload: [[\"er\",...],[\"di\",...],[\"af.httprm\",...]]"
errors: "Gemini API error (HTTP 400 Bad Request): WIZ batchexecute response"
reproduction: "Run live_probe or SDK list_models with fresh cookies; WAA init chain fails on sJBwce batchexecute step, falls back, then list_models batchexecute returns WIZ 400"
started: "unknown / current SDK behavior"

## Eliminated

## Evidence

- timestamp: "2026-08-11T00:02:00Z"
  checked: "src/client.rs list_models, batchexecute_rpc, build_headers, send_batchexecute_with_retry"
  found: "list_models uses source-path /app, sends x-goog-ext-525001261-jspb only when waa_context is set, calls build_batchexecute_body with session.access_token. Client uses cookie_store(true) and also sets manual Cookie header. send_batchexecute_with_retry generates fresh _reqid but does not replace the captured params value."
  implication: "Multiple candidate divergences; need live request capture to identify which one matters"
- timestamp: "2026-08-11T00:35:00Z"
  checked: "/tmp standalone probe with same fresh cookies"
  found: "Standalone probe (cookie_store true/false, source-path /app or /, with or without at token) all return HTTP 200 with valid model list. /app HTML returned at=None (no SNlM0e token found in this HTML shape)."
  implication: "The exact standalone request shape works; the anti-bot trigger must be something specific the SDK does beyond these obvious params. Need to instrument the SDK itself to capture the live request it sends."
- timestamp: "2026-08-11T01:10:00Z"
  checked: "SDK live_probe output"
  found: "Only list_models fails with WIZ 400; all other batchexecute endpoints (user_info, locale_tools, model_config, etc.) succeed. WAA init chain fails on sJBwce as known."
  implication: "The failure is list_models-specific, not a global auth/header issue."
- timestamp: "2026-08-11T01:25:00Z"
  checked: "Temporary modifications: cookie_store(false), regenerating _reqid into params, source-path variations"
  found: "None of these changes fixed list_models."
  implication: "Root cause is not cookie_store conflict, stale _reqid, or source-path. Likely something else in list_models' request construction."
- timestamp: "2026-08-11T02:00:00Z"
  checked: "Attempted to instrument SDK to dump request; eprintln and std::fs::write inside async closure did not produce output/file"
  implication: "The closure is not being executed in the expected path? Or list_models is short-circuited before the request? Need to verify list_models actually reaches the request code."
- timestamp: "2026-08-11T10:45:00Z"
  checked: "Instrumented init_session and run_waa_init_chain; dumped batchexecute_rpc requests"
  found: "list_models reaches ensure_session, which calls init_session. init_session runs run_waa_init_chain. The first otAQ7b warm-up returns HTTP 200, but the second sJBwce batchexecute returns HTTP 400 WIZ. The error propagates and aborts session init. Standalone reqwest probe with the same sJBwce params also returns 400. Git history shows commit 2e4b392 made run_waa_init_chain failures fatal."
  implication: "The public list_models error is actually the propagated sJBwce warm-up failure, not a list_models-specific issue. run_waa_init_chain should tolerate non-WAA warm-up batchexecute failures."

## Resolution

root_cause: "commit 2e4b392 changed run_waa_init_chain to surface failures, but it made the non-WAA batchexecute warm-up steps (otAQ7b, sJBwce, ESY5D) fatal as well. The sJBwce step returns a WIZ 400 for this cookie set, which now aborts session init before any public API method can run. The public list_models error is actually the propagated sJBwce failure, not a list_models-specific anti-bot rejection. A nested session lock in list_models would also deadlock once init succeeds."
fix: "Tolerate failures from the batchexecute warm-up RPCs (otAQ7b, sJBwce, ESY5D) inside run_waa_init_chain, and fall back to a synthetic WAA context when ogads GetAsyncData fails. Keep WAA Create failures as AttestationFailed. Reuse the existing session guard in list_models to avoid a nested lock."
verification: "probe_external and live_probe now pass list_models; full cargo test suite passes; clippy clean."
files_changed:
  - src/client.rs
  - tests/auth_provider.rs
  - tests/integration_tests.rs
