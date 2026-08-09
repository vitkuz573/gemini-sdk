//! Exponential-backoff retry helper for transient failures.

use std::future::Future;
use std::time::Duration;

use backoff::{future::retry, ExponentialBackoff};

/// Initial retry interval for exponential backoff.
const INITIAL_INTERVAL: Duration = Duration::from_millis(500);
/// Maximum retry interval between attempts.
const MAX_INTERVAL: Duration = Duration::from_secs(8);
/// Total wall-clock time before giving up on retries.
const MAX_ELAPSED_TIME: Duration = Duration::from_secs(30);

/// Retries an async operation using exponential backoff.
///
/// The operation receives a fresh clone of the client/request context each time
/// via the provided closure, so it must be cheaply cloneable.
///
/// Backoff parameters:
/// - initial interval: 500 ms
/// - maximum interval: 8 s
/// - maximum elapsed time: 30 s
pub(crate) async fn with_backoff<F, Fut, T>(operation: F) -> crate::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = std::result::Result<T, reqwest::Error>>,
{
    let backoff = ExponentialBackoff {
        initial_interval: INITIAL_INTERVAL,
        max_interval: MAX_INTERVAL,
        max_elapsed_time: Some(MAX_ELAPSED_TIME),
        ..Default::default()
    };

    retry(backoff, || async {
        match operation().await {
            Ok(value) => Ok(value),
            Err(err) => {
                let sdk_err = crate::errors::Error::Request(err);
                if sdk_err.is_transient() {
                    Err(backoff::Error::transient(sdk_err))
                } else {
                    Err(backoff::Error::permanent(sdk_err))
                }
            }
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use reqwest::StatusCode;

    use super::*;
    use crate::Error;

    #[test]
    fn is_transient_public_api() {
        // Transient HTTP status codes.
        assert!(Error::api(StatusCode::TOO_MANY_REQUESTS, "rate limited").is_transient());
        assert!(Error::api(StatusCode::INTERNAL_SERVER_ERROR, "server error").is_transient());
        assert!(Error::api(StatusCode::BAD_GATEWAY, "bad gateway").is_transient());
        assert!(Error::api(StatusCode::SERVICE_UNAVAILABLE, "unavailable").is_transient());
        assert!(Error::api(StatusCode::GATEWAY_TIMEOUT, "gateway timeout").is_transient());

        // Transient error variants.
        assert!(Error::Transient("network".to_string()).is_transient());
        assert!(Error::RateLimited("too many".to_string()).is_transient());
        assert!(Error::Timeout("deadline".to_string()).is_transient());

        // Permanent errors and 4xx (except 429) are not retried.
        assert!(!Error::api(StatusCode::BAD_REQUEST, "bad request").is_transient());
        assert!(!Error::api(StatusCode::NOT_FOUND, "not found").is_transient());
        assert!(!Error::BadRequest("invalid".to_string()).is_transient());
    }

    fn build_reqwest_error(status: StatusCode) -> reqwest::Error {
        // Build a reqwest error carrying the desired HTTP status.
        let response = http::Response::builder()
            .status(status)
            .body("")
            .expect("valid response");
        let response: reqwest::Response = response.into();
        response.error_for_status().expect_err("status is an error")
    }

    #[tokio::test]
    async fn with_backoff_retries_transient_errors() {
        let attempts = Arc::new(AtomicUsize::new(0));

        let operation = {
            let attempts = Arc::clone(&attempts);
            move || -> futures::future::Ready<std::result::Result<&'static str, reqwest::Error>> {
                let attempts = Arc::clone(&attempts);
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    futures::future::ready(Err(build_reqwest_error(StatusCode::TOO_MANY_REQUESTS)))
                } else {
                    futures::future::ready(Ok("success"))
                }
            }
        };

        let result = with_backoff(operation).await;
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn with_backoff_does_not_retry_permanent_4xx() {
        let attempts = Arc::new(AtomicUsize::new(0));

        let operation = {
            let attempts = Arc::clone(&attempts);
            move || -> futures::future::Ready<std::result::Result<&'static str, reqwest::Error>> {
                let attempts = Arc::clone(&attempts);
                attempts.fetch_add(1, Ordering::SeqCst);
                futures::future::ready(Err(build_reqwest_error(StatusCode::BAD_REQUEST)))
            }
        };

        let err = with_backoff(operation).await.unwrap_err();
        assert!(
            matches!(err, Error::Request(ref e) if e.status() == Some(StatusCode::BAD_REQUEST))
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}
