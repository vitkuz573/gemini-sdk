//! Integration tests for browserless WAA slot-3 token generation.

use std::collections::HashMap;

use gemini_sdk::waa::{WaaGenerator, WrapperFragment};
use gemini_sdk::Result;

#[derive(Debug, serde::Deserialize)]
struct Fixture {
    raw_token: String,
    captures: Vec<Capture>,
}

#[derive(Debug, serde::Deserialize)]
struct Capture {
    slot_3: String,
    prompt: String,
    g: String,
    cid: String,
    prqid: String,
    prsid: String,
}

fn fixtures() -> Fixture {
    serde_json::from_str(include_str!("waa_fixtures.json")).expect("fixtures parse")
}

#[test]
fn qh_computation_matches_captures() {
    let fx = fixtures();
    for cap in &fx.captures {
        assert_eq!(WaaGenerator::compute_qh(&cap.prompt, &cap.g), expected_qh(cap));
    }
}

fn expected_qh(cap: &Capture) -> String {
    // The slot_3 value starts with '!' and then contains the base64url
    // payload. We cannot directly compare it to qh; instead we use the
    // precomputed qh from the captures. The fixture file stores slot_3 only,
    // so we derive qh from the captured payload metadata key. For the two
    // known captures these are the validated values below.
    match cap.prompt.as_str() {
        "кто ты" => {
            "5c9abef82e06591cd3cf77e0651bf9ba4d8da58f028ce713004dbbad3be00658".to_string()
        }
        "что тут изображено?" => {
            "72c715b8fcce39f64346aded7f2397fd50281d95274e0ae498616bbffb90e403".to_string()
        }
        _ => panic!("unrecognized test prompt"),
    }
}

#[test]
fn default_generator_reproduces_first_capture() {
    let fx = fixtures();
    let gen = WaaGenerator::bundled().expect("default cache should load");
    let cap = &fx.captures[0];

    let got = gen
        .generate(&fx.raw_token, &cap.prompt, &cap.g, &cap.cid, &cap.prqid, &cap.prsid)
        .expect("first capture should generate");

    assert_eq!(got, cap.slot_3);
}

#[test]
fn default_generator_reproduces_second_capture() {
    let fx = fixtures();
    let gen = WaaGenerator::bundled().expect("default cache should load");
    let cap = &fx.captures[1];

    let got = gen
        .generate(&fx.raw_token, &cap.prompt, &cap.g, &cap.cid, &cap.prqid, &cap.prsid)
        .expect("second capture should generate");

    assert_eq!(got, cap.slot_3);
}

#[test]
fn unknown_signature_reports_missing_wrapper() {
    let fx = fixtures();
    let gen = WaaGenerator::bundled().expect("default cache should load");

    let err = gen
        .generate(
            &fx.raw_token,
            "some unseen prompt",
            "00000000-0000-0000-0000-000000000000",
            "",
            "",
            "",
        )
        .expect_err("unknown signature should fail");

    let msg = format!("{err}");
    assert!(
        msg.contains("unknown WAA metadata signature"),
        "error should describe missing signature: {msg}"
    );
    assert!(msg.contains("attestation failed"), "error should be AttestationFailed: {msg}");
}

#[test]
fn add_signature_allows_custom_cache() -> Result<()> {
    let cache = serde_json::to_string(&custom_cache()).expect("custom cache serializes");
    let mut gen = WaaGenerator::from_json(&cache)?;

    assert!(gen.has_signature("deadbeef", "c", "p", "r"));
    gen.add_signature("cafebabe", "c2", "p2", "r2", "0000000000", "0123456789abcdef")?;
    assert!(gen.has_signature("cafebabe", "c2", "p2", "r2"));

    Ok(())
}

#[test]
fn malformed_cache_rejected() {
    let err = WaaGenerator::from_json("not json").expect_err("invalid JSON should fail");
    assert!(format!("{err}").contains("configuration error"));
}

fn custom_cache() -> HashMap<String, WrapperFragment> {
    let mut map = HashMap::new();
    map.insert(
        serde_json::json!(["deadbeef", "c", "p", "r"]).to_string(),
        WrapperFragment {
            header: "c3c0a5c0a4".to_string(),
            metadata_block: "1a0200000121520000001a".to_string(),
        },
    );
    map
}
