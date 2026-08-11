# Phase 19 Research: Payload & Parser Alignment for `jSf9Qc`

## Source of Truth

Captured browser HAR entry:

```
POST https://gemini.google.com/_/BardChatUi/data/batchexecute?rpcids=jSf9Qc&source-path=%2Fusage&bl=boq_assistant-bard-web-server_20260807.01_p1&f.sid=5670978011641272859&hl=ru&_reqid=6480408&rt=c
```

### Request payload

Decoded `f.req`:

```json
[[["jSf9Qc","[]",null,"generic"]]]
```

Current SDK `build_get_usage_stats_payload()` returns `[]`, which serializes to the same `"[]"` string inside `build_batchexecute_body_for_rpc`. The request shape already matches the captured browser payload.

### Response payload

Browser response (after XSSI prefix):

```
[["wrb.fr","jSf9Qc","[2,[[999999,0,5,null,null,[[1786440269,701531000],2]],[47284,0.02271571,2,[[1786440269,701531000]]],[2333,0.03,1,[[1786220669,701297000]]]],false]",null,null,null,"generic"],["di",167],["af.httprm",167,"-5542029280678317581",7]]
```

Inner payload JSON:

```json
[2,[[999999,0,5,null,null,[[1786440269,701531000],2]],[47284,0.02271571,2,[[1786440269,701531000]]],[2333,0.03,1,[[1786220669,701297000]]]],false]
```

Interpretation:

- Top-level array: `[version, data_array, boolean]`
- `version` = `2` (integer)
- `data_array` contains three sub-arrays. Each sub-array appears to represent a usage bucket:
  - `[count_or_id, unknown_float, unknown_int, null, null, [timestamp_microseconds, unknown_int]]`
- The boolean trailing value is `false`.

Given the goal is to expose daily and total request counts, the most likely candidates are:

- `999999` in the first bucket — possibly a synthetic total/limit.
- `47284` in the second bucket — total requests.
- `2333` in the third bucket — requests today.

However, these semantics are speculative. The safest public API is to expose:

- `UsageStats::requests_today()` → `Option<u64>`
- `UsageStats::requests_total()` → `Option<u64>`

with a fallback to raw `value()` for drift tolerance.

## Current SDK State

- `src/settings.rs` defines `UsageStats` with only `value()` accessor.
- `parse_usage_stats_response` handles null payloads and bare string payloads, plus wrapped arrays.
- Existing test fixture `tests/fixtures/jSf9Qc_usage_stats.txt` returns a synthetic object `{"requests_today":12,"requests_total":345}` which does **not** match the live array shape.

## Decisions for Phase 19

1. Keep `build_get_usage_stats_payload()` returning `[]` — the request shape already matches the HAR.
2. Update the parser to unwrap the array-shaped response and expose typed accessors.
3. Replace the `jSf9Qc_usage_stats.txt` fixture with a HAR-derived one.
4. Preserve `value()` as the drift-tolerant escape hatch.
5. Treat null payloads as empty object (existing behavior).
6. Use `Option<u64>` for accessors because field semantics are undocumented.

## Open Questions

- Exact meaning of the second/third bucket values is unconfirmed; accessors will be conservative.
- Whether the boolean flag at the end is meaningful for callers is unknown; ignore for now.

## References

- HAR file: `/home/vitaly/mitm.har` entry 508
- SDK parser: `src/settings.rs`
- SDK integration test: `tests/integration_tests.rs` `get_usage_stats_returns_value`
