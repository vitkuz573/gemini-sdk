//! Authentication primitives for the Gemini web frontend.
//!
//! The Gemini web frontend uses browser cookies for authentication. The minimum
//! signed-in cookies are `__Secure-1PSID` and `__Secure-1PSIDCC`.

use std::collections::HashMap;
use std::fmt;

/// A collection of cookies used to authenticate with `gemini.google.com`.
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    inner: HashMap<String, String>,
}

impl Cookies {
    /// Creates an empty cookie jar.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
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
        self.get("__Secure-1PSID").is_some() && self.get("__Secure-1PSIDCC").is_some()
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
    pub(crate) fn merge_response_cookies<'a>(&mut self, cookies: impl Iterator<Item = reqwest::cookie::Cookie<'a>>) {
        for cookie in cookies {
            self.inner
                .insert(cookie.name().to_string(), cookie.value().to_string());
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
        write!(f, "{}", self.to_header_value())
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
        cookies.insert("__Secure-1PSID", "x");
        assert!(!cookies.is_signed_in());
        cookies.insert("__Secure-1PSIDCC", "y");
        assert!(cookies.is_signed_in());
    }

    #[test]
    fn header_value_is_sorted_and_formatted() {
        let mut cookies = Cookies::new();
        cookies.insert("z", "last");
        cookies.insert("a", "first");
        assert_eq!(cookies.to_header_value(), "a=first; z=last");
    }
}
