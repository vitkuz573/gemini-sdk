# Spike Manifest

## Idea

Reverse-engineer the current Gemini web frontend protocol from a real 40 MB HAR capture so the Rust SDK can talk to Gemini directly without browser automation.

## Requirements

- `StreamGenerate` must use a 97-slot `inner_req_list` that matches the live frontend.
- Session initialization must obtain WAA/attestation context (`x-goog-ext-525001261-jspb`) without Chrome automation.
- Model listing must use the correct `batchexecute` RPC id (`otAQ7b`).
- Upload flow must remain compatible with `push.clients6.google.com/upload` resumable uploads.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | gemini-protocol | standard | Given a 40 MB HAR capture, compare request/response shapes with the SDK and identify concrete mismatches | ✓ VALIDATED | gemini, protocol, har, reverse-engineering, stream-generate |
