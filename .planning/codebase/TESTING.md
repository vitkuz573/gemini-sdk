# Testing Patterns

**Analysis Date:** 2026-08-08

## Test Framework

**Runner:**
- Built-in Rust test harness (`cargo test`).
- Async tests use `#[tokio::test]` from `tokio-test` 0.4.
- Config: no custom test harness; default `cargo test` behavior.

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, and `panic!` macros.
- `Result::is_err()` / `Result::unwrap()` for error cases in tests.

**Run Commands:**
```bash
cargo test                         # Run all unit + integration tests
cargo test -- --ignored          # Run tests requiring live GEMINI_COOKIES
cargo test --test proto_tests      # Run protocol tests only
cargo test --test integration_tests # Run integration tests only
cargo test --test real_cookies     # Run live-cookie tests (skip if env missing)
cargo bench                        # Run criterion benchmarks
cargo test --features browser-attestation  # Include attestation module tests
cargo test --features capture-fixtures     # Include fixture-capture helpers
```

## Test File Organization

**Location:**
- Inline `#[cfg(test)]` modules co-located with source for unit tests.
- Separate `tests/` directory for integration and live tests.

**Naming:**
- Inline tests: no special naming beyond descriptive function names (`extract_prompt_from_text_message`).
- Integration files: `<scope>_tests.rs`.

**Structure:**
```
tests/
├── integration_tests.rs   # High-level API tests
├── proto_tests.rs         # Protocol parsing and slot building
└── real_cookies.rs        # Live integration tests
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptive_test_name() {
        let input = ...;
        assert_eq!(function(input), expected);
    }
}
```

This pattern appears in `src/auth.rs`, `src/chat.rs`, `src/session.rs`, `src/models.rs`, and `src/proto/*.rs`.

**Patterns:**
- Use `include_str!("path/to/fixture")` for large captured payloads.
- Inline string literals for small, focused inputs.
- `#[tokio::test]` for async tests; `#[ignore = "requires live GEMINI_COOKIES"]` for tests that need real credentials.

## Mocking

**Framework:** None currently used.

**Patterns:**
- `wiremock` 0.6 is listed in dev-dependencies but no test uses it.
- Tests rely on static fixtures in `tests/fixtures/` rather than live HTTP mocking.
- Live HTTP tests in `tests/real_cookies.rs` skip gracefully when `GEMINI_COOKIES` is not set.

**What to Mock:**
- When adding tests for HTTP-dependent logic, prefer `wiremock` or a trait-based abstraction over the `reqwest::Client`.

**What NOT to Mock:**
- Pure protocol parsing and slot-building functions; feed them fixture files.

## Fixtures and Factories

**Test Data:**
- Fixtures are plain text / JSON files stored in `tests/fixtures/`.
- Examples:
  - `tests/fixtures/wiz_global_data.txt` — full `/app` WIZ global data block.
  - `tests/fixtures/turn1_response_raw.txt` — real first-turn StreamGenerate response.
  - `tests/fixtures/model_list_response.txt` — captured model list.
  - `tests/fixtures/thinking_response_raw.txt` — response with reasoning text.

**Location:**
- `tests/fixtures/` for integration tests.
- Inline strings for unit tests.

## Coverage

**Requirements:**
- No explicit coverage target enforced.

**View Coverage:**
```bash
cargo tarpaulin --out Html   # if cargo-tarpaulin is installed
```

## Test Types

**Unit Tests:**
- Co-located in source files under `#[cfg(test)]`.
- Cover cookie parsing, prompt extraction, slot building, model category mapping, session HTML extraction, response parsing.
- Examples: `src/auth.rs:426`, `src/proto/slots.rs:199`, `src/proto/parser.rs:557`.

**Integration Tests:**
- `tests/proto_tests.rs` exercises the full parsing and slot-building pipeline against fixtures.
- `tests/integration_tests.rs` tests public API ergonomics and runs two `#[ignore]` live tests.
- `tests/real_cookies.rs` runs live calls to Google services when `GEMINI_COOKIES` is set.

**E2E Tests:**
- Not used as automated E2E; `tests/real_cookies.rs` and the examples serve as manual E2E validation.

## Common Patterns

**Async Testing:**
```rust
#[tokio::test]
async fn live_text_chat() {
    let cookies = std::env::var("GEMINI_COOKIES").expect("GEMINI_COOKIES required");
    let client = GeminiClient::from_cookie_header(&cookies).unwrap();
    let response = client.chat().send_message("Hi").await.unwrap();
    assert!(!response.text().is_empty());
}
```
(See `tests/integration_tests.rs:40`.)

**Error Testing:**
```rust
#[test]
fn credentials_validate_requires_required_cookies() {
    let mut creds = Credentials::new();
    assert_eq!(creds.validate().unwrap_err(), CredentialsError::MissingPsid);
    creds.psid = "x".to_string();
    assert_eq!(creds.validate().unwrap_err(), CredentialsError::MissingPsidcc);
}
```
(See `src/auth.rs:469`.)

**Fixture-Based Parsing Test:**
```rust
#[test]
fn parse_real_response_fixture() {
    let body = include_str!("fixtures/turn1_response_raw.txt");
    let response = parse_chat_response(body).unwrap();
    assert!(!response.text().is_empty());
}
```
(See `tests/proto_tests.rs:104`.)

---

*Testing analysis: 2026-08-08*
