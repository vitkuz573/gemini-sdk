---
name: spike-findings-gemini-sdk
description: Implementation blueprint from spike experiments. Requirements, proven patterns, and verified knowledge for building gemini-sdk. Auto-loaded during implementation work.
---

<context>
## Project: gemini-sdk

A clean, well-structured, production-ready Rust SDK for interacting with the Google Gemini / Bard web frontend (`gemini.google.com`). The SDK is built by reverse-engineering the undocumented web frontend protocol from MITM HAR captures.

Spike sessions wrapped: 2026-08-08
</context>

<requirements>
## Requirements

- `StreamGenerate` must use a 97-slot `inner_req_list` that matches the live frontend.
- Session initialization must obtain WAA/attestation context (`x-goog-ext-525001261-jspb`) without Chrome automation.
- Model listing must use the correct `batchexecute` RPC id (`otAQ7b`).
- Upload flow must remain compatible with `push.clients6.google.com/upload` resumable uploads.
- The `bl` (build label) query parameter must be extracted from the live `/app` HTML (`window.WIZ_global_data.cfb2h`), not hardcoded.
</requirements>

<findings_index>
## Feature Areas

| Area | Reference | Key Finding |
|------|-----------|-------------|
| Protocol | references/protocol.md | 97-slot StreamGenerate, batchexecute RPC ids, response parsing, upload flow |
| WAA / Attestation | references/waa-attestation.md | Initialization chain, `x-goog-ext-525001261-jspb` template, SAPISIDHASH |
| Auth | references/auth.md | Signed-in detection, SNlM0e extraction, required cookie set |

## Source Files

Original spike source files are preserved in `sources/` for complete reference.
</findings_index>

<metadata>
## Processed Spikes

- 001-gemini-protocol
- 002-gemini-protocol
- 003-gemini-protocol
- 004-waa-token
- 005-snlM0e
- 006-signed-in-detection
- 007-build-label
- 008-cookie-auth
- 009-har-api-coverage
</metadata>
