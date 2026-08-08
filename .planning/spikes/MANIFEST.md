# Spike Manifest

## Idea

Reverse-engineer the current Gemini web frontend protocol from a real 40 MB HAR capture so the Rust SDK can talk to Gemini directly without browser automation.

## Requirements

- `StreamGenerate` must use a 97-slot `inner_req_list` that matches the live frontend.
- Session initialization must obtain WAA/attestation context (`x-goog-ext-525001261-jspb`) without Chrome automation.
- Model listing must use the correct `batchexecute` RPC id (`otAQ7b`).
- Upload flow must remain compatible with `push.clients6.google.com/upload` resumable uploads.
- The `bl` (build label) query parameter must be extracted from the live `/app` HTML (`window.WIZ_global_data.cfb2h`), not hardcoded.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | gemini-protocol | standard | Given a 40 MB HAR capture, compare request/response shapes with the SDK and identify concrete mismatches | ✓ VALIDATED | gemini, protocol, har, reverse-engineering, stream-generate |
| 002 | gemini-protocol | standard | Compare new `/full1.har` capture with spike 001 and SDK to confirm protocol changes | ✓ VALIDATED | gemini, protocol, har, reverse-engineering, stream-generate, waa |
| 003 | gemini-protocol | standard | Given a fresh 119 MB HAR with full response bodies, extract exact shapes and make the SDK work end-to-end without browser automation | ✓ VALIDATED | gemini, protocol, har, reverse-engineering, stream-generate, waa, ogads, upload |
| 004 | waa-token | standard | Reverse-engineer the BotGuard WAA token that goes into StreamGenerate slot 3 using only captured artifacts, without browser automation | IN PROGRESS | gemini, waa, botguard, reverse-engineering, attestation, slot-3 |
| 005 | snlM0e | standard | Determine how the Gemini frontend extracts and consumes `SNlM0e` (the `at` token) from `window.WIZ_global_data` | ✓ VALIDATED | gemini, snlM0e, at, wiz, auth |
| 006 | signed-in-detection | standard | Determine how the frontend decides signed-in state and how the SDK should implement `is_signed_in` | ✓ VALIDATED | gemini, auth, wiz, signed-in |
| 007 | build-label | standard | Given the Gemini /app HTML response, automatically extract the current build label (`bl`) from `window.WIZ_global_data.cfb2h` instead of hardcoding it | ✓ VALIDATED | gemini, protocol, build-label, bl, wiz |
