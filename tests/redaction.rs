//! Integration test: `Credentials` must not leak any secret material in its
//! `Debug` output.

use gemini_sdk::auth::{Credentials, APISID, PAPISID, PSID, PSIDCC, PSIDTS, SAPISID, SOCS};

/// Returns a `Credentials` instance whose every secret field is populated with a
/// synthetic, easily-detected value.
fn credentials_with_secrets() -> Credentials {
    let mut creds = Credentials::new();
    creds.psid = "secret-psid-value".to_string();
    creds.psidcc = "secret-psidcc-value".to_string();
    creds.psidts = Some("secret-psidts-value".to_string());
    creds.papisid = Some("secret-papisid-value".to_string());
    creds.sapisid = Some("secret-sapisid-value".to_string());
    creds.apisid = Some("secret-apisid-value".to_string());
    creds.socs = Some("secret-socs-value".to_string());
    creds
        .extra
        .insert("extra-secret-name".to_string(), "extra-secret-value".to_string());
    creds
}

#[test]
fn debug_contains_no_secret_substrings() {
    let creds = credentials_with_secrets();
    let debug = format!("{creds:?}");

    let secrets = [
        "secret-psid-value",
        "secret-psidcc-value",
        "secret-psidts-value",
        "secret-papisid-value",
        "secret-sapisid-value",
        "secret-apisid-value",
        "secret-socs-value",
        "extra-secret-value",
    ];

    for secret in &secrets {
        assert!(
            !debug.contains(secret),
            "Debug output leaked secret substring: {secret}\n{debug}"
        );
    }
}

#[test]
fn debug_redacts_non_empty_secrets() {
    let creds = credentials_with_secrets();
    let debug = format!("{creds:?}");

    // Each named secret field should render as "<redacted>".
    for field in ["psid", "psidcc", "psidts", "papisid", "sapisid", "apisid", "socs"] {
        // Optional fields are wrapped in `Some(...)` by `Debug`.
        let expected_bare = format!("{field}: \"<redacted>\"");
        let expected_some = format!("{field}: Some(\"<redacted>\")");
        assert!(
            debug.contains(&expected_bare) || debug.contains(&expected_some),
            "expected {field} to be redacted in\n{debug}"
        );
    }
}

#[test]
fn debug_shows_empty_secrets() {
    let mut creds = Credentials::new();
    creds.psid = "".to_string();
    creds.psidcc = "".to_string();

    let debug = format!("{creds:?}");

    assert!(
        debug.contains("psid: \"(empty)\"") || debug.contains("psid: Some(\"(empty)\")"),
        "expected '(empty)' for psid in\n{debug}"
    );
    assert!(
        debug.contains("psidcc: \"(empty)\"") || debug.contains("psidcc: Some(\"(empty)\")"),
        "expected '(empty)' for psidcc in\n{debug}"
    );
}

#[test]
fn debug_only_counts_extra_cookies() {
    let mut creds = Credentials::new();
    creds.psid = "a".to_string();
    creds.psidcc = "b".to_string();
    creds.extra.insert("x".to_string(), "x-value".to_string());
    creds.extra.insert("y".to_string(), "y-value".to_string());

    let debug = format!("{creds:?}");

    // The extra cookie *values* must not leak even though they live in an
    // untyped map.
    assert!(!debug.contains("x-value"), "extra cookie value leaked\n{debug}");
    assert!(!debug.contains("y-value"), "extra cookie value leaked\n{debug}");
    // Only the count is exposed.
    assert!(debug.contains("extra: 2"), "expected extra cookie count in\n{debug}");
}

#[test]
fn cookie_header_round_trip_still_works() {
    let header = format!(
        "{PSID}=psid-value; {PSIDCC}=psidcc-value; {PSIDTS}=ts; {PAPISID}=papi; {SAPISID}=sapi; {APISID}=api; {SOCS}=consent"
    );
    let creds = Credentials::from_header(&header).unwrap();
    assert_eq!(creds.psid, "psid-value");
    assert_eq!(creds.psidcc, "psidcc-value");
    assert_eq!(creds.psidts.as_deref(), Some("ts"));
    assert_eq!(creds.papisid.as_deref(), Some("papi"));
    assert_eq!(creds.sapisid.as_deref(), Some("sapi"));
    assert_eq!(creds.apisid.as_deref(), Some("api"));
    assert_eq!(creds.socs.as_deref(), Some("consent"));
}
