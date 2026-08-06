//! Exponential-backoff retry helper for transient failures.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use backoff::{future::retry, ExponentialBackoff};
use tokio::sync::Mutex;

/// Retries an async operation using exponential backoff.
///
/// The operation receives a fresh clone of the client/request context each time
/// via the provided closure, so it must be cheaply cloneable.
pub(crate) async fn with_backoff<F, Fut, T>(operation: F) -> crate::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = std::result::Result<T, reqwest::Error>>,
{
    let backoff = ExponentialBackoff {
        initial_interval: Duration::from_millis(500),
        max_interval: Duration::from_secs(8),
        max_elapsed_time: Some(Duration::from_secs(30)),
        ..Default::default()
    };

    let operation = Arc::new(Mutex::new(operation));

    retry(backoff, || {
        let operation = Arc::clone(&operation);
        async move {
            let op = operation.lock().await;
            let fut = (*op)();
            drop(op);
            match fut.await {
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
        }
    })
    .await
}
