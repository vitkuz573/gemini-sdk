# Requirements: Gemini SDK

**Defined:** 2026-08-11
**Core Value:** Developers can reliably integrate Gemini into Rust applications using a stable, documented, semver-respecting SDK that handles auth, protocol quirks, retries, and common content types out of the box.

## Validated (v0.1 Core + v0.2 API Expansion)

For shipped requirements see `.planning/milestones/v0.2-REQUIREMENTS.md`.

## Active (v0.3 Magic String Elimination)

### Core Protocol Constants

- [ ] **MAINT-01**: URL path constants (`/app`, `/app?hl={}`, `/_/BardChatUi/data/batchexecute`, `/usage`, `/scheduled`, `/app/{id}`) are centralized and referenced by name instead of inline strings.
- [ ] **MAINT-02**: batchexecute query parameter keys (`rpcids`, `source-path`, `hl`, `_reqid`, `rt`, `bl`, `f.sid`) are centralized as named constants.
- [ ] **MAINT-03**: batchexecute transport strings (`wrb.fr`, XSSI prefix, `f.req`, `batchexecute` endpoint discriminator) are centralized as named constants.
- [ ] **MAINT-04**: WIZ/session extraction keys (`S06Grb`, `oPEP7c`, `FdrFJe`, `cfb2h`, `f.sid`) are centralized as named constants.
- [ ] **MAINT-05**: RPC identifier constants are complete and consistently named for every batchexecute RPC (including `otAQ7b` / `Fd0Qje`).

### Model, Chat & Upload Constants

- [ ] **MAINT-06**: Model category enum values, display strings, and fallback derivation keywords are centralized and not duplicated.
- [ ] **MAINT-07**: Chat message roles (`user`, `model`) are constants; no inline role strings remain in production code.
- [ ] **MAINT-08**: MIME type constants for supported images, audio, video, PDF, and upload media validation are centralized.
- [ ] **MAINT-09**: Upload endpoint strings (`push.clients6.google.com`, upload command headers, `x-goog-upload-*` header names, tenant id) are centralized.

### Infrastructure Constants

- [ ] **MAINT-10**: Base URL constants (`gemini.google.com`, WAA, OGADS, push) are centralized.
- [ ] **MAINT-11**: Static header names and values (`x-client-data`, `x-goog-ext-*`, `sec-ch-ua-*`, `User-Agent`, `Origin`, `Referer`, cache pragmas, fetch metadata) are centralized.
- [ ] **MAINT-12**: HAR/redaction strings (HAR version, creator name, MIME types, secret header names, cookie names, redaction pattern list) are centralized.
- [ ] **MAINT-13**: Transient WIZ 400 markers (`er`, `di`, `af.httprm`) and NotSignedIn diagnostics strings are centralized.
- [ ] **MAINT-14**: Tracing / metrics operation names and metric names are centralized as constants.
- [ ] **MAINT-15**: Browser attestation CDP method names, selectors, and default strings are centralized.
- [ ] **MAINT-16**: Tool schema keys (`type`, `properties`, `required`, `name`, `parameters`) are centralized.

### Test & Example Cleanup

- [ ] **MAINT-17**: Magic strings in tests and examples are replaced by constants from the source modules they exercise; shared test constants live in a `tests/common` module if needed.
- [ ] **MAINT-18**: A regression gate (clippy lint or dedicated test) prevents new literal occurrences of the eliminated magic strings in `src/`.
- [ ] **MAINT-19**: `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` pass after all v0.3 changes.

## Future

- **AUTH-V2-01**: OAuth / refresh-token flow as an alternative to cookie strings.
- **MEDIA-V2-01**: Resumable upload with explicit chunk size control.
- **PROTO-V2-01**: Schema-aware WIZ payload validation before sending.
- **PROTO-V2-02**: Automatic protocol drift detection from live HAR captures.
- **ADV-V2-01**: Batch / async tool execution with parallel tool calls.
- **ADV-V2-02**: Conversation branching and history pruning.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Telemetry / heartbeat RPCs (`aPya6c`, `maGuAc`, `GPRiHf`, `I4z33b`, `VxUbXb`, `MyzX6c`, `qpEbW`) | Library SDK should not emit analytics traffic to Google. |
| Config/state rollout RPCs (`CNgdBe`, `Bsxleb`, `ozz5Z`) | Internal feature-rollout / state payloads; not user-facing. |
| Reporting endpoints (`cspreport/fine-allowlist`, `jserror`, `web-reports`) | Browser UI reporting; not applicable to a server-side SDK. |
| `signaler-pa` and `myactivity.google.com` endpoints | Real-time signalling and history APIs; out of scope for library SDK. |
| Official REST / Vertex AI client | This SDK intentionally targets the undocumented web frontend protocol. |
| Real-time voice / video calls | Requires WebRTC or a different transport; not a chat SDK concern. |
| Mobile platform bindings | Out of scope for a Rust crate; could be a separate FFI wrapper. |
| Quota / billing management | Owned by Google; SDK only wraps frontend access. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| MAINT-01 | Phase 13 | Not started |
| MAINT-02 | Phase 13 | Not started |
| MAINT-03 | Phase 13 | Not started |
| MAINT-04 | Phase 13 | Not started |
| MAINT-05 | Phase 13 | Not started |
| MAINT-06 | Phase 14 | Not started |
| MAINT-07 | Phase 14 | Not started |
| MAINT-08 | Phase 14 | Not started |
| MAINT-09 | Phase 14 | Not started |
| MAINT-10 | Phase 15 | Not started |
| MAINT-11 | Phase 15 | Not started |
| MAINT-12 | Phase 15 | Not started |
| MAINT-13 | Phase 15 | Not started |
| MAINT-14 | Phase 15 | Not started |
| MAINT-15 | Phase 15 | Not started |
| MAINT-16 | Phase 15 | Not started |
| MAINT-17 | Phase 16 | Not started |
| MAINT-18 | Phase 16 | Not started |
| MAINT-19 | Phase 16 | Not started |

**Coverage:**

- v0.3 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

---
*Requirements defined: 2026-08-11 for milestone v0.3*
