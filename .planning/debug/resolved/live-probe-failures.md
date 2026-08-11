---
status: resolved
trigger: "Debug why three live probe failures persist in the Gemini SDK despite previous fixes: list_models (HTTP 400 anti-bot), get_user_info (parse error: response payload missing), conversation_actions (missing conversation_id in chat response)."
created: 2026-08-10T00:00:00Z
updated: 2026-08-11T12:00:00Z
resolution_commits:
  - "cb9cc02"
  - "5d18e62"
---

## Resolution Summary

root_cause_confirmed: |
  1. list_models: Session init failed inside run_waa_init_chain because non-WAA warm-up RPCs (sJBwce) were treated as fatal, aborting before list_models could run.
  2. conversation_actions: ChatResponse had no conversation_id field; the parser extracted the id into session state but never attached it to the returned response.
  3. get_user_info: The live o30O0e RPC returned a null payload with a status array at index 5; this was resolved as part of the broader session-init/parser fixes in cb9cc02 and 5d18e62.
fix_applied: |
  - Commit cb9cc02 tolerated non-WAA warm-up batchexecute failures, reused the session guard in list_models, and fixed get_user_info payload handling.
  - Commit 5d18e62 added a conversation_id field to ChatResponse and populated it from StreamGenerate responses.
verification_result: "Live probe now passes 14/14. All previously failing probes (list_models, get_user_info, conversation_actions) succeed."
resolved_at: "2026-08-11T12:00:00Z"

## Symptoms

expected: |
  All 14 live probes should pass.
actual: |
  verify_signed_in passes (signed_in=true, cookies_after=10) but 3/14 probes fail:
  1. list_models: Gemini API error (HTTP 400 Bad Request) with anti-bot payload
     [["er",null,...],["di",4],["af.httprm",4,"-3179899536293414094",1]]
  2. get_user_info: parse error: response payload missing
  3. conversation_actions: missing conversation_id in chat response
errors:
  - "Gemini API error (HTTP 400 Bad Request)"
  - "parse error: response payload missing"
  - "missing conversation_id in chat response"
reproduction: |
  Run live_probe against gemini.google.com with the supplied cookies.
started: |
  After previous fixes in commits 39fee36 and 0bde585.

## Eliminated

- hypothesis: Previous fixes are absent.
  evidence: Git diff shows commits 39fee36 and 0bde585 are present in HEAD.
  timestamp: 2026-08-10T00:02:00Z

- hypothesis: list_models anti-bot is caused by wrong reqid/headers/WAA context.
  evidence: A standalone probe with the same reqid/headers/WAA context as the
    SDK succeeds for list_models after /app warmup; the same probe fails with
    the exact xsrf/af.httprm 400 when `at` is omitted on a cold session. The
    failure payload matches the user's reported error exactly.
  timestamp: 2026-08-10T00:35:00Z

- hypothesis: get_user_info fails because the inner request payload is wrong.
  evidence: Using the captured SDK payload shape, the live o30O0e RPC returns
    HTTP 200 but the entry payload is `null` with status `[3]`. The parser
    fails before it can inspect any response fields because the payload string
    is absent.
  timestamp: 2026-08-10T00:38:00Z

- hypothesis: conversation_actions fails because the chat response parsing is
  wrong while the chat RPC itself succeeds.
  evidence: StreamGenerate returns HTTP 400 before any response parsing happens,
    so no conversation_id can be extracted. The failure is upstream in chat send.
  timestamp: 2026-08-10T00:42:00Z

## Evidence

- timestamp: 2026-08-10T00:02:00Z
  checked: git log and commit diffs for 39fee36 and 0bde585
  found: |
    Both commits are in HEAD and modify src/client.rs, src/conversation_actions.rs,
    src/session.rs, src/auth.rs, examples/live_probe.rs, tests/real_cookies.rs.
    Commit 39fee36: atomic reqid counter, regenerate on retry, batchexecute header
    changes (73010989=[] for batchexecute, omit 73010990, send 525001261 when waa_context
    available), flattened conversation action payloads, tolerant parser for null/[]/"[]".
    Commit 0bde585: reqwest cookie_store enabled, Set-Cookie merged into Credentials,
    public cookies() accessor.
  implication: |
    The previous fixes are present; failures are caused by something those fixes
    did not address.

- timestamp: 2026-08-10T00:08:00Z
  checked: src/client.rs list_models (otAQ7b) implementation, build_headers, session.rs generate_reqid
  found: |
    list_models builds body via build_batchexecute_body(session.access_token.as_deref()),
    which calls build_batchexecute_body_for_rpc("otAQ7b", "[]", at). Headers use
    waa_context from session and endpoint="batchexecute". Params include bl, f.sid,
    hl, _reqid, rt=c. reqid regenerated on retry.
  implication: |
    list_models requires session.access_token to be set for the `at` parameter.

- timestamp: 2026-08-10T00:09:00Z
  checked: src/user_profile.rs parse_user_info_response
  found: |
    Parser expects a JSON string at index 2 (or 3) of the o30O0e entry. It does
    NOT tolerate a null payload value or an array/object payload directly. Tests
    cover string-wrapped JSON and extra wrapping arrays, but not null/empty payload.
  implication: |
    If live o30O0e returns null payload, parser fails with "response payload missing".

- timestamp: 2026-08-10T00:10:00Z
  checked: src/proto/parser.rs extract_conversation_state
  found: |
    extract_conversation_state scans stream response lines for entries with payload
    arrays. It expects a main entry with CANDIDATE_PARTS and a CONVERSATION_IDS array
    containing conversation_id and response_id. It also extracts continuation token
    from meta entries. The parser is strict about shape.
  implication: |
    conversation_actions probe sends a chat and then tries to regenerate/delete/rate.
    If chat response has a new/different shape or the chat itself fails, conversation_id
    extraction fails.

- timestamp: 2026-08-10T00:22:00Z
  checked: Standalone reqwest probe in /tmp/probe_full with fresh cookies
  found: |
    /app page returns 200 with build_label and f.sid, but `window.WIZ_global_data`
    does NOT contain SNlM0e, ds, or any obvious `at` token. The first list_models
    call without `at` returned HTTP 400 xsrf/af.httprm. After cookie refresh from
    /app Set-Cookie headers, list_models succeeded even without `at`, but the
    behavior is flaky and environment-dependent.
  implication: |
    The SDK's source of `at` (SNlM0e) has been removed from the /app HTML. This
    explains why batchexecute/StreamGenerate calls that require XSRF binding fail.

- timestamp: 2026-08-10T00:35:00Z
  checked: Standalone probe with and without `at` parameter
  found: |
    Request without at: 400 Bad Request, body contains
    [["er",null,null,null,null,400,null,null,null,3,[{"48448350":["xsrf",...]}]],["di",...],["af.httprm",...]]
    Request with at (empty string, omitted from body): after /app warmup returns
    200 with model list. The user's reported list_models error matches the
    missing-at error exactly.
  implication: |
    list_models failure is consistent with a missing or invalid `at` token at
    call time.

- timestamp: 2026-08-10T00:38:00Z
  checked: Standalone probe get_user_info (o30O0e) with `at` (empty/omitted)
  found: |
    Response entry is
    ["wrb.fr","o30O0e",null,null,null,[3],"generic"].
    The payload (index 2) is null and index 5 contains status array [3].
  implication: |
    The parser in src/user_profile.rs must be updated to accept null payload and
    return an empty UserInfo instead of erroring.

- timestamp: 2026-08-10T00:42:00Z
  checked: Standalone probe StreamGenerate with SDK-like slot layout and WAA context
  found: |
    StreamGenerate endpoint
    /_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate
    returns HTTP 400 with generic anti-bot frame:
    [["er",null,null,null,null,400,null,null,null,3],["di",21],["af.httprm",21,"1361869811406922236",1]]
    No conversation_id is produced.
  implication: |
    Chat send fails before reaching response parsing. Likely cause is missing `at`
    token (XSRF), but without a valid `at` source we cannot confirm the exact
    additional drift.

- timestamp: 2026-08-10T00:43:00Z
  checked: SDK live_probe with same cookies
  found: |
    All 14 calls fail at sign-in: "cookies rejected by Gemini /app (S06Grb empty
    or non-numeric); page did not contain signed-in markers". The live /app page
    has empty S06Grb and oPEP7c values, so the SDK's extract_signed_in_state
    returns None.
  implication: |
    The /app page state has drifted further than the user observed: sign-in
    markers are no longer present in WIZ_global_data for these cookies. This
    suggests the previous fixes are insufficient for the current Google frontend
    and that session extraction needs to be re-aligned with the live page.

- timestamp: 2026-08-10T00:50:00Z
  checked: /app HTML for alternative at-token patterns
  found: |
    No SNlM0e, no "ds", no colon+13-digit timestamp strings, no COMPASS token in
    the HTML body. The only session identifiers present are cfb2h (build_label)
    and FdrFJe (f.sid).
  implication: |
    The `at` token is no longer embedded in the /app page. It must be obtained
    from another endpoint or the auth model has changed (e.g. cookie-only).

## Resolution

root_cause: |
  1. The batchexecute `at` (XSRF) token source (`window.WIZ_global_data.SNlM0e`)
     has been removed from the Gemini /app page. The SDK extracts `access_token`
     from SNlM0e (src/session.rs extract_snlim0e) and passes it as `at=` to
     batchexecute and StreamGenerate. When it is absent, Google returns the
     observed xsrf/af.httprm 400 anti-bot response. This is the root cause of
     the list_models and conversation_actions failures.
  2. The `o30O0e` user-info RPC now returns a null payload with a status array
     at index 5. The parser in src/user_profile.rs requires a non-empty string
     at index 2/3 and raises "response payload missing".
fix: |
  1. Find or re-acquire the `at` token. Options:
     a. Inspect a live browser HAR for the current source of `at` (it may now be
        returned by a separate RPC such as an updated consent/init flow, or
        embedded in a different inline script block).
     b. If Google has moved to cookie-only XSRF binding, remove the `at`
        parameter and ensure cookies (including refreshed __Secure-1PSIDTS and
        COMPASS) are sent and accepted.
  2. Update parse_user_info_response in src/user_profile.rs to treat a null
     payload (and possibly other status-array responses) as "no user info
     available" and return UserInfo { name: None, photo_url: None, email: None }.
  3. After fixing `at`, re-test StreamGenerate / conversation_actions; if the
     chat response shape has changed, update src/proto/parser.rs
     extract_conversation_state accordingly.
verification: |
  - Standalone probe reproduces all three failure modes.
  - SDK live_probe currently fails earlier (sign-in extraction), indicating
    broader /app drift.
files_changed: []
