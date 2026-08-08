# Spike Wrap-Up Summary

**Date:** 2026-08-08
**Spikes processed:** 9
**Feature areas:** protocol, waa-attestation, auth
**Skill output:** `./.opencode/skills/spike-findings-gemini-sdk/`

## Processed Spikes

| # | Name | Type | Verdict | Feature Area |
|---|------|------|---------|--------------|
| 001 | gemini-protocol | standard | VALIDATED | protocol |
| 002 | gemini-protocol | standard | VALIDATED | protocol |
| 003 | gemini-protocol | standard | VALIDATED | protocol |
| 004 | waa-token | standard | IN PROGRESS | waa-attestation |
| 005 | snlM0e | standard | PENDING | auth |
| 006 | signed-in-detection | standard | VALIDATED | auth |
| 007 | build-label | standard | VALIDATED | protocol |
| 008 | cookie-auth | standard | PENDING | auth |
| 009 | har-api-coverage | standard | VALIDATED | protocol |

## Key Findings

- The Gemini web frontend protocol is reverse-engineerable from MITM HAR captures.
- `StreamGenerate` uses a 97-slot JSON array; the SDK implementation matches the captured shape for fresh and continuation turns.
- Core chat flow requires `/app` bootstrap, `otAQ7b` model list, WAA/ogads attestation chain, and `StreamGenerate`.
- `x-goog-ext-525001261-jspb` can be assembled from a fixed template using the Pro model fingerprint and request UUID.
- Signed-in detection must inspect `window.WIZ_global_data.S06Grb` and `oPEP7c`, not just cookie presence.
- Many `batchexecute` RPCs observed in traffic are UI telemetry/settings/history calls and are not required for a server-side SDK.
- The only notable drift against the latest 135 MB HAR is the `x-client-data` header constant.
