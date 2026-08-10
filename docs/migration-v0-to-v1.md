# Migrating from v0.x to v1.0

This guide documents breaking changes introduced in the v0.1.0 release and the
path toward v1.0.0. The v1.0 version bump itself is deferred to a future
milestone; this document is current as of the v0.1.0 release.

## Async config builder methods

`GeminiClient` configuration builder methods are now `async` because the
underlying `ClientConfig` is protected by a `tokio::sync::RwLock`.

### Before

```rust,ignore
let client = GeminiClient::from_cookie_header(cookies)?
    .with_language("ru")
    .with_max_retries(5)
    .with_timeout(Duration::from_secs(60));
```

### After

```rust,ignore
let client = GeminiClient::from_cookie_header(cookies)?
    .with_language("ru").await
    .with_max_retries(5).await
    .with_timeout(Duration::from_secs(60)).await;
```

All of the following methods are affected:

- `GeminiClient::from_provider` (was already async)
- `GeminiClient::with_provider`
- `GeminiClient::with_language`
- `GeminiClient::with_max_retries`
- `GeminiClient::with_timeout`
- `GeminiClient::with_system_instruction`
- `GeminiClient::with_http_hook`
- `GeminiClient::with_fatal_hook_errors`
- `GeminiClient::with_metrics`

## Typed attestation errors

WAA/ogads attestation failures now surface as `Error::AttestationFailed`
instead of silently falling back to a synthetic context. Callers that need to
proceed despite attestation failures must catch this variant explicitly.

### Before

```rust,ignore
let client = GeminiClient::from_cookie_header(cookies)?;
// Attestation failures were logged and ignored; the client continued
// with a best-effort synthetic context.
```

### After

```rust,ignore
let client = GeminiClient::from_cookie_header(cookies)?;
match client.init_session().await {
    Ok(()) => {}
    Err(gemini_sdk::Error::AttestationFailed { reason }) => {
        eprintln!("attestation failed: {reason}");
        // Decide whether to retry, use a fallback, or abort.
    }
    Err(e) => return Err(e),
}
```

## v1.0 semver commitment

When v1.0.0 is released, the crate will follow strict SemVer:

- Breaking changes will only occur in major version bumps.
- Minor versions will add functionality backwards-compatibly.
- Patch versions will contain only bug fixes.

Until then, minor 0.x bumps may include additional breaking changes as the API
continues to mature.
