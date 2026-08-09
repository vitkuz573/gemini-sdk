//! Authentication primitives for the Gemini web frontend.
//!
//! The Gemini web frontend uses browser cookies for authentication. The minimum
//! signed-in cookies are `__Secure-1PSID` and `__Secure-1PSIDCC`.

use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

/// Name of the primary session cookie.
pub const PSID: &str = "__Secure-1PSID";
/// Name of the secondary signed-in cookie.
pub const PSIDCC: &str = "__Secure-1PSIDCC";
/// Name of the timestamp / anti-replay cookie.
pub const PSIDTS: &str = "__Secure-1PSIDTS";
/// Name of the signed-in PAPISID cookie used for SAPISIDHASH.
pub const PAPISID: &str = "__Secure-1PAPISID";
/// Name of the legacy `APISID` cookie.
pub const APISID: &str = "APISID";
/// Name of the legacy `SAPISID` cookie.
pub const SAPISID: &str = "SAPISID";
/// Consent-state cookie.
pub const SOCS: &str = "SOCS";

/// Errors that can occur while validating or building credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialsError {
    /// The primary session cookie is missing.
    MissingPsid,
    /// The secondary signed-in cookie is missing.
    MissingPsidcc,
}

impl fmt::Display for CredentialsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPsid => write!(f, "missing required cookie `{PSID}`"),
            Self::MissingPsidcc => write!(f, "missing required cookie `{PSIDCC}`"),
        }
    }
}

impl std::error::Error for CredentialsError {}

/// A strongly-typed set of Google cookies used to authenticate with
/// `gemini.google.com`.
///
/// `Credentials` stores cookies by their semantic role rather than as an opaque
/// header string. Secrets are redacted from [`Debug`] output.
#[derive(Clone, Default)]
pub struct Credentials {
    /// Primary session ID.
    pub psid: String,
    /// Secondary signed-in token.
    pub psidcc: String,
    /// Optional timestamp / anti-replay cookie.
    pub psidts: Option<String>,
    /// Optional signed-in PAPISID value.
    pub papisid: Option<String>,
    /// Optional legacy SAPISID value.
    pub sapisid: Option<String>,
    /// Optional legacy APISID value.
    pub apisid: Option<String>,
    /// Optional consent-state cookie.
    pub socs: Option<String>,
    /// Any additional cookies that were provided (e.g. `HSID`, `SSID`, `NID`).
    pub extra: HashMap<String, String>,
}

impl Credentials {
    /// Creates empty credentials.
    ///
    /// An empty credential set is not valid; use [`validate`][Self::validate]
    /// before sending requests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds credentials from a raw `Cookie` header value.
    ///
    /// # Example
    ///
    /// ```
    /// use gemini_sdk::auth::{Credentials, PSID, PSIDCC};
    ///
    /// let creds = Credentials::from_header("__Secure-1PSID=abc; __Secure-1PSIDCC=def").unwrap();
    /// assert_eq!(creds.psid, "abc");
    /// assert_eq!(creds.psidcc, "def");
    /// ```
    pub fn from_header(header: &str) -> Result<Self, CredentialsError> {
        let mut creds = Self::new();
        for pair in header.split(';') {
            let mut it = pair.trim().splitn(2, '=');
            let name = it.next().map(str::trim).unwrap_or("");
            let value = it.next().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            match name {
                PSID => creds.psid = value,
                PSIDCC => creds.psidcc = value,
                PSIDTS => creds.psidts = Some(value),
                PAPISID => creds.papisid = Some(value),
                SAPISID => creds.sapisid = Some(value),
                APISID => creds.apisid = Some(value),
                SOCS => creds.socs = Some(value),
                _ => {
                    creds.extra.insert(name.to_string(), value);
                }
            }
        }
        creds.validate()?;
        Ok(creds)
    }

    /// Returns true if the credential set looks like it belongs to a signed-in
    /// session. This is a fast local check: it requires the primary session
    /// cookies, a SAPISID source, and the legacy `SID`, `HSID`, `SSID` auth
    /// cookies.
    ///
    /// For a server-authoritative check, use
    /// [`crate::client::GeminiClient::verify_signed_in`].
    #[must_use]
    pub fn is_signed_in(&self) -> bool {
        self.has_cookie_credentials()
            && self.sapisid_value().is_some()
            && self.has_legacy_auth_cookies()
    }

    /// Returns true if the minimum signed-in cookies are present.
    ///
    /// This is the backward-compatible semantics of the old `is_signed_in`:
    /// only `__Secure-1PSID` and `__Secure-1PSIDCC` are required.
    #[must_use]
    pub fn has_cookie_credentials(&self) -> bool {
        !self.psid.is_empty() && !self.psidcc.is_empty()
    }

    /// Returns true if the legacy `SID`, `HSID`, and `SSID` cookies are present.
    #[must_use]
    pub fn has_legacy_auth_cookies(&self) -> bool {
        ["SID", "HSID", "SSID"].iter().all(|name| self.extra.contains_key(*name))
    }

    /// Returns true if the credentials include the timestamp cookie that some
    /// endpoints (e.g. `StreamGenerate`) require.
    #[must_use]
    pub fn has_streaming_credentials(&self) -> bool {
        self.is_signed_in() && self.psidts.is_some()
    }

    /// Validates that the credential set is usable for signed-in requests.
    pub fn validate(&self) -> Result<(), CredentialsError> {
        if self.psid.is_empty() {
            return Err(CredentialsError::MissingPsid);
        }
        if self.psidcc.is_empty() {
            return Err(CredentialsError::MissingPsidcc);
        }
        Ok(())
    }

    /// Returns the SAPISID value used for `Authorization: SAPISIDHASH`.
    ///
    /// Prefers the signed-in `__Secure-1PAPISID` cookie, then falls back to
    /// `SAPISID` / `APISID`.
    #[must_use]
    pub fn sapisid_value(&self) -> Option<&str> {
        self.papisid
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.sapisid.as_deref().filter(|s| !s.is_empty()))
            .or_else(|| self.apisid.as_deref().filter(|s| !s.is_empty()))
    }

    /// Computes an `Authorization: SAPISIDHASH <ts>_<sha1>` header value.
    ///
    /// The hash is `SHA1(<timestamp> <sapisid> <origin>)` as used by Google
    /// grpc-web endpoints (ogads, waa). The trailing slash on the origin must
    /// be omitted to match the browser's computation.
    #[must_use]
    pub fn sapisid_hash(&self, origin: &str) -> Option<String> {
        use sha1::{Digest, Sha1};

        let sapisid = self.sapisid_value()?;
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let origin = origin.strip_suffix('/').unwrap_or(origin);
        let input = format!("{ts} {sapisid} {origin}");
        let digest = Sha1::digest(input.as_bytes());
        let hex = format!("{digest:x}");
        Some(format!("SAPISIDHASH {ts}_{hex}"))
    }

    /// Serialises the credentials into a `Cookie` header value.
    #[must_use]
    pub(crate) fn to_header_value(&self) -> String {
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        pairs.push((PSID, self.psid.as_str()));
        pairs.push((PSIDCC, self.psidcc.as_str()));
        if let Some(v) = self.psidts.as_deref() {
            pairs.push((PSIDTS, v));
        }
        if let Some(v) = self.papisid.as_deref() {
            pairs.push((PAPISID, v));
        }
        if let Some(v) = self.sapisid.as_deref() {
            pairs.push((SAPISID, v));
        }
        if let Some(v) = self.apisid.as_deref() {
            pairs.push((APISID, v));
        }
        if let Some(v) = self.socs.as_deref() {
            pairs.push((SOCS, v));
        }

        let mut extra: Vec<(&String, &String)> = self.extra.iter().collect();
        extra.sort_by(|a, b| a.0.cmp(b.0));
        pairs.extend(extra.iter().map(|(k, v)| (k.as_str(), v.as_str())));

        pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Merges cookies from a `Set-Cookie` style iterator.
    pub(crate) fn merge_response_cookies<'a>(
        &mut self,
        cookies: impl Iterator<Item = reqwest::cookie::Cookie<'a>>,
    ) {
        for cookie in cookies {
            let name = cookie.name().to_string();
            let value = cookie.value().to_string();
            match name.as_str() {
                PSID => self.psid = value,
                PSIDCC => self.psidcc = value,
                PSIDTS => self.psidts = Some(value),
                PAPISID => self.papisid = Some(value),
                SAPISID => self.sapisid = Some(value),
                APISID => self.apisid = Some(value),
                SOCS => self.socs = Some(value),
                _ => {
                    self.extra.insert(name, value);
                }
            }
        }
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn redact(v: &str) -> String {
            if v.is_empty() {
                String::from("(empty)")
            } else {
                String::from("<redacted>")
            }
        }
        f.debug_struct("Credentials")
            .field("psid", &redact(&self.psid))
            .field("psidcc", &redact(&self.psidcc))
            .field("psidts", &self.psidts.as_deref().map(redact))
            .field("papisid", &self.papisid.as_deref().map(redact))
            .field("sapisid", &self.sapisid.as_deref().map(redact))
            .field("apisid", &self.apisid.as_deref().map(redact))
            .field("socs", &self.socs.as_deref().map(redact))
            .field("extra", &self.extra.len())
            .finish()
    }
}

impl TryFrom<&str> for Credentials {
    type Error = CredentialsError;

    fn try_from(header: &str) -> Result<Self, Self::Error> {
        Self::from_header(header)
    }
}

impl TryFrom<String> for Credentials {
    type Error = CredentialsError;

    fn try_from(header: String) -> Result<Self, Self::Error> {
        Self::from_header(&header)
    }
}

/// Async provider of [`Credentials`].
///
/// Implement this trait to source credentials from environment variables, files,
/// keyrings, or other external stores without changing the simple
/// [`crate::GeminiClient::from_cookie_header`] constructor.
///
/// A boxed-future signature is used to keep the trait object-safe and avoid an
/// extra `async-trait` runtime dependency.
pub trait CredentialsProvider: Send + Sync {
    /// Resolves the credentials used to authenticate requests.
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>>;
}

impl CredentialsProvider for Credentials {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>> {
        let creds = self.clone();
        Box::pin(async move { Ok(creds) })
    }
}

/// Default provider that parses a `Cookie` header string on demand.
#[derive(Debug, Clone)]
pub struct CookieHeaderProvider {
    header: String,
}

impl CookieHeaderProvider {
    /// Creates a provider from a raw `Cookie` header value.
    ///
    /// # Errors
    ///
    /// Returns an error if the header is missing required cookies.
    pub fn new(header: impl Into<String>) -> Result<Self, CredentialsError> {
        let header = header.into();
        // Validate eagerly so construction fails fast with a typed error.
        let _ = Credentials::from_header(&header)?;
        Ok(Self { header })
    }
}

impl CredentialsProvider for CookieHeaderProvider {
    fn credentials(&self) -> Pin<Box<dyn Future<Output = crate::Result<Credentials>> + Send + '_>> {
        let header = self.header.clone();
        Box::pin(async move {
            Credentials::from_header(&header)
                .map_err(|e| crate::Error::Config(e.to_string()))
        })
    }
}

/// A flexible cookie jar used internally for backward compatibility.
///
/// Prefer [`Credentials`] for new code. `Cookies` is kept as a thin wrapper
/// around a map for cases where callers want to manipulate arbitrary cookies.
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    inner: HashMap<String, String>,
}

impl Cookies {
    /// Creates an empty cookie jar.
    pub fn new() -> Self {
        Self { inner: HashMap::new() }
    }

    /// Parses a raw `Cookie` header value such as the one copied from a browser.
    ///
    /// # Example
    ///
    /// ```
    /// use gemini_sdk::auth::Cookies;
    ///
    /// let cookies = Cookies::from_header("__Secure-1PSID=abc; __Secure-1PSIDCC=def");
    /// assert_eq!(cookies.get("__Secure-1PSID"), Some("abc"));
    /// ```
    pub fn from_header(header: &str) -> Self {
        let mut inner = HashMap::new();
        for pair in header.split(';') {
            let mut it = pair.trim().splitn(2, '=');
            let name = it.next().map(str::trim).unwrap_or("");
            let value = it.next().unwrap_or("");
            if !name.is_empty() {
                inner.insert(name.to_string(), value.to_string());
            }
        }
        Self { inner }
    }

    /// Inserts or replaces a cookie.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.inner.insert(name.into(), value.into());
    }

    /// Returns the value of a cookie, if present.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner.get(name).map(String::as_str)
    }

    /// Removes a cookie from the jar.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.inner.remove(name)
    }

    /// Returns `true` if the jar contains the minimum signed-in cookies.
    #[must_use]
    pub fn is_signed_in(&self) -> bool {
        self.get(PSID).is_some() && self.get(PSIDCC).is_some()
    }

    /// Serialises the cookies into a `Cookie` header value.
    pub(crate) fn to_header_value(&self) -> String {
        let mut pairs: Vec<(&String, &String)> = self.inner.iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(b.0));
        pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Merges cookies from a `Set-Cookie` style iterator.
    pub(crate) fn merge_response_cookies<'a>(
        &mut self,
        cookies: impl Iterator<Item = reqwest::cookie::Cookie<'a>>,
    ) {
        for cookie in cookies {
            self.inner.insert(cookie.name().to_string(), cookie.value().to_string());
        }
    }
}

impl From<HashMap<String, String>> for Cookies {
    fn from(map: HashMap<String, String>) -> Self {
        Self { inner: map }
    }
}

impl From<Cookies> for HashMap<String, String> {
    fn from(cookies: Cookies) -> Self {
        cookies.inner
    }
}

impl Extend<(String, String)> for Cookies {
    fn extend<T: IntoIterator<Item = (String, String)>>(&mut self, iter: T) {
        self.inner.extend(iter);
    }
}

impl fmt::Display for Cookies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} cookies>", self.inner.len())
    }
}

impl Cookies {
    /// Builds `Credentials` from the cookies in the jar.
    pub fn to_credentials(&self) -> Result<Credentials, CredentialsError> {
        Credentials::from_header(&self.to_header_value())
    }
}

impl From<Credentials> for Cookies {
    fn from(value: Credentials) -> Self {
        let mut cookies = Cookies::new();
        cookies.insert(PSID, value.psid);
        cookies.insert(PSIDCC, value.psidcc);
        if let Some(v) = value.psidts {
            cookies.insert(PSIDTS, v);
        }
        if let Some(v) = value.papisid {
            cookies.insert(PAPISID, v);
        }
        if let Some(v) = value.sapisid {
            cookies.insert(SAPISID, v);
        }
        if let Some(v) = value.apisid {
            cookies.insert(APISID, v);
        }
        if let Some(v) = value.socs {
            cookies.insert(SOCS, v);
        }
        cookies.extend(value.extra);
        cookies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let cookies = Cookies::from_header("a=1; b=2; c=3");
        assert_eq!(cookies.get("a"), Some("1"));
        assert_eq!(cookies.get("b"), Some("2"));
        assert_eq!(cookies.get("c"), Some("3"));
    }

    #[test]
    fn signed_in_requires_both_cookies() {
        let mut cookies = Cookies::new();
        assert!(!cookies.is_signed_in());
        cookies.insert(PSID, "x");
        assert!(!cookies.is_signed_in());
        cookies.insert(PSIDCC, "y");
        assert!(cookies.is_signed_in());
    }

    #[test]
    fn header_value_is_sorted_and_formatted() {
        let mut cookies = Cookies::new();
        cookies.insert("z", "last");
        cookies.insert("a", "first");
        assert_eq!(cookies.to_header_value(), "a=first; z=last");
    }

    #[test]
    fn credentials_parse_known_fields() {
        let header = format!(
            "{PSID}=psid-value; {PSIDCC}=psidcc-value; {PSIDTS}=ts; {PAPISID}=papi; {SOCS}=consent"
        );
        let creds = Credentials::from_header(&header).unwrap();
        assert_eq!(creds.psid, "psid-value");
        assert_eq!(creds.psidcc, "psidcc-value");
        assert_eq!(creds.psidts.as_deref(), Some("ts"));
        assert_eq!(creds.papisid.as_deref(), Some("papi"));
        assert_eq!(creds.socs.as_deref(), Some("consent"));
    }

    #[test]
    fn credentials_validate_requires_required_cookies() {
        let mut creds = Credentials::new();
        assert_eq!(creds.validate().unwrap_err(), CredentialsError::MissingPsid);
        creds.psid = "x".to_string();
        assert_eq!(creds.validate().unwrap_err(), CredentialsError::MissingPsidcc);
        creds.psidcc = "y".to_string();
        assert!(creds.validate().is_ok());
    }

    #[test]
    fn credentials_is_signed_in_requires_sapisid_and_legacy_auth() {
        let mut creds = Credentials::new();
        assert!(!creds.is_signed_in());
        creds.psid = "psid".to_string();
        assert!(!creds.is_signed_in());
        creds.psidcc = "psidcc".to_string();
        // has_cookie_credentials is true, but SAPISID and legacy auth are missing.
        assert!(creds.has_cookie_credentials());
        assert!(!creds.is_signed_in());

        creds.papisid = Some("papisid".to_string());
        assert!(!creds.is_signed_in());

        creds.extra.insert("SID".to_string(), "sid".to_string());
        creds.extra.insert("HSID".to_string(), "hsid".to_string());
        creds.extra.insert("SSID".to_string(), "ssid".to_string());
        assert!(creds.is_signed_in());

        // SAPISID via legacy fallback also works.
        creds.papisid = None;
        creds.sapisid = Some("sapisid".to_string());
        assert!(creds.is_signed_in());
        creds.sapisid = None;
        creds.apisid = Some("apisid".to_string());
        assert!(creds.is_signed_in());
    }

    #[test]
    fn credentials_has_streaming_credentials_requires_psidts() {
        let header =
            format!("{PSID}=a; {PSIDCC}=b; {PAPISID}=c; {PSIDTS}=d; SID=s; HSID=h; SSID=s");
        let creds = Credentials::from_header(&header).unwrap();
        assert!(creds.has_streaming_credentials());

        let header_no_ts = format!("{PSID}=a; {PSIDCC}=b; {PAPISID}=c; SID=s; HSID=h; SSID=s");
        let creds_no_ts = Credentials::from_header(&header_no_ts).unwrap();
        assert!(!creds_no_ts.has_streaming_credentials());
    }

    #[test]
    fn credentials_to_header_value_orders_known_cookies_first() {
        let header = format!("extra_z=last; {PSID}=a; extra_a=first; {PSIDCC}=b; {PAPISID}=c");
        let creds = Credentials::from_header(&header).unwrap();
        let value = creds.to_header_value();
        assert!(value.starts_with(&format!("{PSID}=a; {PSIDCC}=b; {PAPISID}=c")));
        assert!(value.contains("extra_a=first; extra_z=last"));
    }

    #[test]
    fn cookies_display_redacts_secrets() {
        let mut cookies = Cookies::new();
        cookies.insert("__Secure-1PSID", "secret-psid");
        cookies.insert("__Secure-1PSIDCC", "secret-psidcc");
        let display = format!("{}", cookies);
        assert!(!display.contains("secret-psid"));
        assert_eq!(display, "<2 cookies>");
    }

    #[test]
    fn credentials_debug_redacts_secrets() {
        let header = format!("{PSID}=secret-psid; {PSIDCC}=secret-psidcc; {PAPISID}=secret-papi");
        let creds = Credentials::from_header(&header).unwrap();
        let debug = format!("{creds:?}");
        assert!(!debug.contains("secret-psid"));
        assert!(!debug.contains("secret-papi"));
        assert!(debug.contains("\"<redacted>\""));
    }

    #[test]
    fn credentials_from_cookies_round_trips() {
        let header = format!("{PSID}=a; {PSIDCC}=b; {PSIDTS}=c; extra=d");
        let creds = Credentials::from_header(&header).unwrap();
        let cookies: Cookies = creds.into();
        assert_eq!(cookies.get(PSID), Some("a"));
        assert_eq!(cookies.get(PSIDTS), Some("c"));
        assert_eq!(cookies.get("extra"), Some("d"));
    }
}
