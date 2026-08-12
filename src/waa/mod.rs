//! Browserless WAA (Web Abuse and Attribution) token generation.
//!
//! This module reproduces the Google Gemini `StreamGenerate` slot-3 WAA token
//! wrapper without running a browser. The wrapper layout was reverse-engineered
//! from live traffic and validated against captured HAR data.
//!
//! # What is generative
//!
//! - `qh = hex(sha256(textQuery + g))`, where `g` is the UUID placed in slot 59.
//! - Bytes `[5:89]` of the final token come directly from the raw BotGuard VM
//!   token.
//! - Bytes `[89:100]` are structurally constant (`1a 02 00 00 01 21 52 00 00 00`
//!   plus metadata length) but are bundled into the cached metadata block for
//!   simplicity and byte-for-byte reproduction.
//!
//! # What is captured
//!
//! - The 5-byte request header (environment-derived, not a deterministic
//!   function of the signature).
//! - The full metadata block (`[89:]` in the decoded payload), which includes
//!   the fixed prefix, metadata submessage, and the VM-generated tail.
//!
//! Captured wrapper fragments are keyed by `(qh, cid, prqid, prsid)`.
//!
//! # Cache
//!
//! A JSON cache maps a serialized signature `[qh, cid, prqid, prsid]` to an
//! object with two hex fields:
//!
//! - `header`: the 5-byte header as hex.
//! - `metadata_block`: the remainder of the decoded payload from offset 89 as
//!   hex.
//!
//! The crate ships with `src/waa/data/default_wrappers.json`, which contains
//! the two validated signatures from the v0.5 spike.
//!
//! # Limitations
//!
//! Unknown signatures cannot be synthesized. The non-generative parts depend on
//! the live BotGuard VM and browser environment signals. When a signature is
//! missing, [`WaaGenerator::generate`] returns [`Error::AttestationFailed`] and
//! includes the computed `qh` so the caller can capture the missing wrapper.
//!
//! [`Error::AttestationFailed`]: crate::errors::Error::AttestationFailed

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::{Error, Result};

/// The minimum number of bytes expected in a raw BotGuard token.
const RAW_TOKEN_MIN_LEN: usize = 89;

/// The number of bytes copied from the raw token into the slot-3 payload.
const RAW_TOKEN_BODY_LEN: usize = 84;

/// Leading character used by the frontend on base64url-encoded WAA tokens.
const TOKEN_PREFIX: char = '!';

/// A captured wrapper fragment.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WrapperFragment {
    /// The 5-byte request header, stored as lowercase hex.
    pub header: String,
    /// The payload tail from offset 89 to the end, stored as lowercase hex.
    pub metadata_block: String,
}

/// Signature used to look up a captured wrapper fragment.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Signature {
    /// SHA-256 of `textQuery + g`, as 64 lowercase hex characters.
    pub qh: String,
    /// Conversation id.
    pub cid: String,
    /// Previous request id.
    pub prqid: String,
    /// Previous response id.
    pub prsid: String,
}

impl Signature {
    /// Create a signature from its components.
    #[must_use]
    pub fn new(
        qh: impl Into<String>,
        cid: impl Into<String>,
        prqid: impl Into<String>,
        prsid: impl Into<String>,
    ) -> Self {
        Self {
            qh: qh.into(),
            cid: cid.into(),
            prqid: prqid.into(),
            prsid: prsid.into(),
        }
    }

    /// Serialize the signature to the JSON-array form used in cache files.
    #[must_use]
    pub fn to_cache_key(&self) -> String {
        serde_json::json!([&self.qh, &self.cid, &self.prqid, &self.prsid]).to_string()
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "qh={} cid={:?} prqid={:?} prsid={:?}",
            self.qh, self.cid, self.prqid, self.prsid
        )
    }
}

/// Browserless WAA slot-3 token generator.
///
/// Loads a cache of captured wrapper fragments and assembles final tokens from a
/// raw BotGuard VM token, the prompt text, the request UUID, and conversation
/// metadata.
#[derive(Clone, Debug)]
pub struct WaaGenerator {
    fragments: HashMap<Signature, WrapperFragment>,
}

impl WaaGenerator {
    /// Load a generator from a JSON cache file.
    ///
    /// The file must map serialized signature arrays to [`WrapperFragment`]
    /// objects. Passing a non-existent path returns a clear [`Error::Config`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if the file cannot be read or the JSON is
    /// malformed.
    pub fn new(cache_path: impl AsRef<Path>) -> Result<Self> {
        let path = cache_path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!("failed to read WAA cache at {}: {e}", path.display()))
        })?;
        Self::from_json(&text)
    }

    /// Build a generator from an in-memory JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if the JSON cannot be parsed or any fragment
    /// fails validation.
    pub fn from_json(text: &str) -> Result<Self> {
        let raw: HashMap<String, WrapperFragment> = serde_json::from_str(text)
            .map_err(|e| Error::Config(format!("invalid WAA cache JSON: {e}")))?;

        let mut fragments = HashMap::with_capacity(raw.len());
        for (key, frag) in raw {
            let sig = parse_signature_key(&key)?;
            validate_fragment(&sig, &frag)?;
            fragments.insert(sig, frag);
        }

        Ok(Self { fragments })
    }

    /// Build a generator with the bundled default cache.
    ///
    /// This is equivalent to loading [`DEFAULT_CACHE_JSON`] and is the
    /// recommended constructor for the common SDK use case. For the infallible
    /// `Default` trait implementation, see [`Default`] below.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if the bundled cache is corrupt.
    pub fn bundled() -> Result<Self> {
        Self::from_json(DEFAULT_CACHE_JSON)
    }

    /// Compute the prompt hash `qh = hex(sha256(textQuery + g_uuid))`.
    #[must_use]
    pub fn compute_qh(text_query: &str, g_uuid: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text_query.as_bytes());
        hasher.update(g_uuid.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Register a new captured wrapper fragment for a given signature.
    ///
    /// The `header` and `metadata_block` arguments may be provided as hex
    /// strings; the method stores them in normalized lowercase form.
    pub fn add_signature(
        &mut self,
        qh: impl Into<String>,
        cid: impl Into<String>,
        prqid: impl Into<String>,
        prsid: impl Into<String>,
        header: impl Into<String>,
        metadata_block: impl Into<String>,
    ) -> Result<()> {
        let sig = Signature::new(qh, cid, prqid, prsid);
        let frag = WrapperFragment {
            header: header.into().to_lowercase(),
            metadata_block: metadata_block.into().to_lowercase(),
        };
        validate_fragment(&sig, &frag)?;
        self.fragments.insert(sig, frag);
        Ok(())
    }

    /// Generate a base64url slot-3 token with a leading `!`.
    ///
    /// # Arguments
    ///
    /// * `raw_token_b64url` - Base64url-encoded BotGuard VM token, with or
    ///   without the leading `!`.
    /// * `text_query` - The prompt text used to compute `qh`.
    /// * `g_uuid` - The UUID placed in slot 59 of the request.
    /// * `cid`, `prqid`, `prsid` - Conversation / previous request / previous
    ///   response ids.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AttestationFailed`] when the signature is not present
    /// in the cache; the error message includes the computed `qh`.
    pub fn generate(
        &self,
        raw_token_b64url: &str,
        text_query: &str,
        g_uuid: &str,
        cid: &str,
        prqid: &str,
        prsid: &str,
    ) -> Result<String> {
        let raw = decode_b64url(raw_token_b64url)?;
        if raw.len() < RAW_TOKEN_MIN_LEN {
            return Err(Error::AttestationFailed {
                reason: format!("raw token too short: {} bytes", raw.len()),
            });
        }

        let qh = Self::compute_qh(text_query, g_uuid);
        let sig = Signature::new(&qh, cid, prqid, prsid);
        let frag = self.fragments.get(&sig).ok_or_else(|| {
            Error::AttestationFailed {
                reason: format!(
                    "unknown WAA metadata signature: {sig}. The non-generative wrapper must be captured from live traffic first."
                ),
            }
        })?;

        let header = hex::decode(&frag.header).expect("validated header hex");
        let metadata_block = hex::decode(&frag.metadata_block).expect("validated metadata hex");

        let mut payload =
            Vec::with_capacity(header.len() + RAW_TOKEN_BODY_LEN + metadata_block.len());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&raw[5..RAW_TOKEN_MIN_LEN]);
        payload.extend_from_slice(&metadata_block);

        Ok(format!("{TOKEN_PREFIX}{}", URL_SAFE.encode(&payload).trim_end_matches('=')))
    }

    /// Return true if a fragment exists for the given signature.
    #[must_use]
    pub fn has_signature(&self, qh: &str, cid: &str, prqid: &str, prsid: &str) -> bool {
        self.fragments.contains_key(&Signature::new(qh, cid, prqid, prsid))
    }

    /// Return the number of cached fragments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    /// Return true if no fragments are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }
}

impl Default for WaaGenerator {
    /// Creates a generator with the bundled default cache.
    ///
    /// # Panics
    ///
    /// Panics if the bundled cache is corrupt. This should never happen in a
    /// release build; use [`WaaGenerator::bundled`] if error handling is
    /// required.
    fn default() -> Self {
        Self::bundled().expect("bundled WAA cache is corrupt")
    }
}

/// The bundled default cache as a JSON string.
pub const DEFAULT_CACHE_JSON: &str = include_str!("data/default_wrappers.json");

fn parse_signature_key(key: &str) -> Result<Signature> {
    let arr: Vec<String> = serde_json::from_str(key)
        .map_err(|e| Error::Config(format!("invalid WAA cache key {key:?}: {e}")))?;
    if arr.len() != 4 {
        return Err(Error::Config(format!(
            "WAA cache key must have 4 elements [qh, cid, prqid, prsid], got {}: {key:?}",
            arr.len()
        )));
    }
    Ok(Signature::new(arr[0].clone(), arr[1].clone(), arr[2].clone(), arr[3].clone()))
}

fn validate_fragment(sig: &Signature, frag: &WrapperFragment) -> Result<()> {
    if frag.header.len() % 2 != 0 {
        return Err(Error::Config(format!(
            "header hex for signature {} has odd length {}",
            sig.to_cache_key(),
            frag.header.len()
        )));
    }
    if frag.metadata_block.len() % 2 != 0 {
        return Err(Error::Config(format!(
            "metadata_block hex for signature {} has odd length {}",
            sig.to_cache_key(),
            frag.metadata_block.len()
        )));
    }
    if hex::decode(&frag.header).is_err() {
        return Err(Error::Config(format!(
            "header for signature {} is not valid hex",
            sig.to_cache_key()
        )));
    }
    if hex::decode(&frag.metadata_block).is_err() {
        return Err(Error::Config(format!(
            "metadata_block for signature {} is not valid hex",
            sig.to_cache_key()
        )));
    }
    Ok(())
}

fn decode_b64url(input: &str) -> Result<Vec<u8>> {
    let stripped = input.strip_prefix(TOKEN_PREFIX).unwrap_or(input);
    let padded = stripped.replace('-', "+").replace('_', "/");
    let padding = (4 - padded.len() % 4) % 4;
    let padded = format!("{padded}{}", "=".repeat(padding));
    base64::engine::general_purpose::STANDARD.decode(&padded).map_err(|e| {
        Error::AttestationFailed {
            reason: format!("failed to decode raw token: {e}"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_TOKEN_B64URL: &str = "!rK-lr_XNAAYxsMuMEbBCim5LaCArjrE7AEABEArZ1PnRumGQTVaI3inzZMEWqIcw15EsTeN9p3qF0cFmaSN-ksdPjNjDiw35ZhVfR5GJOdsI-wsaiNjCa20aDwAEFTG4OmgBB34AHiqmMPgBuPvL2t4H7MYpD27mV5Hq8bl2HdKsCAg5jAoBnWi9F2Kko4VLGI58OYGoG0Wm0CV2mlShIEy7HkEJZu2EYi4TCZbUIHR3oe4-uow6Q1dXfuVTHgoRZRM8w8g3kwriYJbcqpI5Ga4RVFXNR6nj0tOoIBmHK7R0ag3E3Mj7usVfOn-fTyVKSuq9fyMFssxz9C5JoUBSQcQL_qyFwsNBZjY2d5WIQr2d4nd6Ryoq9ZzIQwIbq_RHXGzsxKB2fTAGVoEQqdWTuHZ0WRa--Su03dSINvo7jbtV4HTeTomsK9066aOMQ_OEORSkBpHk9C0pTBuV5716NsWPZ3bRbn4a9umsYpAEyL7bsGqQm8muLOA-85Brchq2TrO4l4D7o9yWHjZdavmOsfgK2TzaSExmvTcNpKmcaARzVQmT_EGTPSC5RgbeEHdskTX2GaRRrkQmyM9G3u1rayUyu89aIbiL0UgAHiS5k8HWRyFHiTgqhdLgHuOnmEJnut2knAyprftd2o5TRgUkLeVkGfeM2HlWZ4iB6RTQImfDKMtMA8NMP21ir1vRE6dIb7FVnILZdxTyU4-8NOtdc9k88dVBmQC3VAY5IZd-obbNgeIANDbNW246Tm8aNLNpzbOAG4Nj8fnC6SSmJOM8OntHfjfSGaJDkEb0octTxLtgjcYcLIu7Z8K80uzem6sxd-mp0J1wWqKofzcFeVQ7FfJgzRaUIi6aPDJfQ240qi8e-KcDCAOB5Ecpu5mLP2EjsPwIDaqRCaJLN3Zxi6O_BH7SkMs6Nhg58hcQW6_egStvEOwhYpTnVGVCTca_MO-y3cYaF2MR2qcQr6FvskaB";

    #[test]
    fn compute_qh_matches_python() {
        assert_eq!(
            WaaGenerator::compute_qh("кто ты", "59300AF8-2998-47A6-9279-8990CF9A6655"),
            "5c9abef82e06591cd3cf77e0651bf9ba4d8da58f028ce713004dbbad3be00658"
        );
        assert_eq!(
            WaaGenerator::compute_qh("что тут изображено?", "6A7866D2-EA2A-4B39-B02B-0883B31FDA2F"),
            "72c715b8fcce39f64346aded7f2397fd50281d95274e0ae498616bbffb90e403"
        );
    }

    #[test]
    fn default_generator_loads_two_signatures() {
        let gen = WaaGenerator::bundled().unwrap();
        assert_eq!(gen.len(), 2);
    }

    #[test]
    fn generate_reproduces_captures() {
        let fx: serde_json::Value =
            serde_json::from_str(include_str!("test_fixtures.json")).expect("test fixtures parse");
        let raw_token = fx["raw_token"].as_str().unwrap();
        let gen = WaaGenerator::bundled().unwrap();

        for cap in fx["captures"].as_array().unwrap() {
            let got = gen
                .generate(
                    raw_token,
                    cap["prompt"].as_str().unwrap(),
                    cap["g"].as_str().unwrap(),
                    cap["cid"].as_str().unwrap(),
                    cap["prqid"].as_str().unwrap(),
                    cap["prsid"].as_str().unwrap(),
                )
                .unwrap();
            assert_eq!(got, cap["slot_3"].as_str().unwrap());
        }
    }

    #[test]
    fn unknown_signature_returns_error() {
        let gen = WaaGenerator::bundled().unwrap();
        let err = gen
            .generate(
                "!rK-lr_XNAAYxsMuMEbBCim5LaCArjrE7AEABEArZ1PnRumGQTVaI3inzZMEWqIcw15EsTeN9p3qF0cFmaSN-ksdPjNjDiw35ZhVfR5GJOdsI-wsaiNjCa20aDwAEFTG4OmgBB34AHiqmMPgBuPvL2t4H7MYpD27mV5Hq8bl2HdKsCAg5jAoBnWi9F2Kko4VLGI58OYGoG0Wm0CV2mlShIEy7HkEJZu2EYi4TCZbUIHR3oe4-uow6Q1dXfuVTHgoRZRM8w8g3kwriYJbcqpI5Ga4RVFXNR6nj0tOoIBmHK7R0ag3E3Mj7usVfOn-fTyVKSuq9fyMFssxz9C5JoUBSQcQL_qyFwsNBZjY2d5WIQr2d4nd6Ryoq9ZzIQwIbq_RHXGzsxKB2fTAGVoEQqdWTuHZ0WRa--Su03dSINvo7jbtV4HTeTomsK9066aOMQ_OEORSkBpHk9C0pTBuV5716NsWPZ3bRbn4a9umsYpAEyL7bsGqQm8muLOA-85Brchq2TrO4l4D7o9yWHjZdavmOsfgK2TzaSExmvTcNpKmcaARzVQmT_EGTPSC5RgbeEHdskTX2GaRRrkQmyM9G3u1rayUyu89aIbiL0UgAHiS5k8HWRyFHiTgqhdLgHuOnmEJnut2knAyprftd2o5TRgUkLeVkGfeM2HlWZ4iB6RTQImfDKMtMA8NMP21ir1vRE6dIb7FVnILZdxTyU4-8NOtdc9k88dVBmQC3VAY5IZd-obbNgeIANDbNW246Tm8aNLNpzbOAG4Nj8fnC6SSmJOM8OntHfjfSGaJDkEb0octTxLtgjcYcLIu7Z8K80uzem6sxd-mp0J1wWqKofzcFeVQ7FfJgzRaUIi6aPDJfQ240qi8e-KcDCAOB5Ecpu5mLP2EjsPwIDaqRCaJLN3Zxi6O_BH7SkMs6Nhg58hcQW6_egStvEOwhYpTnVGVCTca_MO-y3cYaF2MR2qcQr6FvskaB",
                "unseen prompt",
                "11111111-1111-1111-1111-111111111111",
                "",
                "",
                "",
            )
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown WAA metadata signature"), "unexpected error: {msg}");
        assert!(msg.contains("attestation failed"), "error should be AttestationFailed: {msg}");
        assert!(msg.contains("cef019181d"), "error should include qh prefix: {msg}");
    }

    #[test]
    fn add_signature_round_trips() {
        let mut gen = WaaGenerator::bundled().unwrap();
        gen.add_signature(
            "deadbeef",
            "cid",
            "prqid",
            "prsid",
            "c3c0a5c0a4",
            "1a0200000121520000001a",
        )
        .unwrap();
        assert!(gen.has_signature("deadbeef", "cid", "prqid", "prsid"));
    }

    #[test]
    fn invalid_hex_fragment_rejected() {
        let mut gen = WaaGenerator::bundled().unwrap();
        let err = gen.add_signature("a", "b", "c", "d", "not-hex", "1a0200").unwrap_err();
        assert!(format!("{err}").contains("configuration error"));
    }
}
