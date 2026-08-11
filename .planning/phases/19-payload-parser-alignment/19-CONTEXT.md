---
phase: 19-payload-parser-alignment
name: Payload & Parser Alignment
milestone: v0.5 Usage Stats Reliability
decisions_locked:
  - build_get_usage_stats_payload stays as `[]`; request shape already matches HAR.
  - Parser must handle the array-shaped live response `[2, [...], false]`.
  - UsageStats exposes typed `requests_today()` and `requests_total()` accessors returning Option<u64>.
  - `UsageStats::value()` remains as the drift-tolerant raw escape hatch.
  - Fixture `tests/fixtures/jSf9Qc_usage_stats.txt` must be replaced with a HAR-derived array payload.
  - Null payloads still return an empty JSON object.
scope_fences:
  - Do not change the public `UsageStats` struct name or `value()` method signature.
  - Do not add new crate dependencies.
  - Do not modify request auth headers (handled in phase 18).
requirements:
  - REQ-01
  - PARSER-01
  - PARSER-02
  - PARSER-03
  - API-01
  - API-02
  - TEST-01
  - TEST-04
---

# Phase 19 Context: Payload & Parser Alignment

## Objective

Update the `jSf9Qc` usage-stats response parsing and public API so the SDK
returns real counts from the live array-shaped response while preserving
forward-compatibility against protocol drift.

## Background

Phase 18 added the required auth headers. The request payload already matches
the browser (`[]`). The remaining gap is the parser: it currently expects a
JSON object string like `{"requests_today":12,"requests_total":345}`, but the
live response is an array `[2,[[...],[...],[...]],false]`.

## Locked Decisions

1. **Request payload:** No change. `build_get_usage_stats_payload()` returns `serde_json::json!([])`, which serializes to `"[]"` — identical to the captured browser request.
2. **Response parsing:** The parser must detect the array shape, extract the likely daily/total counts, and fall back to an empty object if the shape is unrecognizable.
3. **Public API:** Add `requests_today() -> Option<u64>` and `requests_total() -> Option<u64>` to `UsageStats`. Keep `value() -> &serde_json::Value` unchanged.
4. **Fixture:** Replace `tests/fixtures/jSf9Qc_usage_stats.txt` with a HAR-derived fixture containing the array payload.
5. **Null behavior:** Preserve existing behavior where a null payload yields an empty JSON object.

## Scope Fences

- No new dependencies.
- No changes to auth headers or `build_headers`.
- No breaking changes to the public `UsageStats` type.

## Dependencies

- Phase 18 (auth headers) must be complete. It is.

## Risks

- The semantic mapping of array buckets to "today" and "total" is inferred from a single HAR capture. Accessors return `Option<u64>` to tolerate drift.
