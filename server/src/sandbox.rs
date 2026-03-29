//! Sandboxed execution with timeout.

use std::time::Duration;

use crate::error::ApiError;

/// Run a blocking closure with a wall-clock timeout.
pub async fn sandboxed<F, T>(f: F, timeout_secs: u64) -> Result<T, ApiError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        tokio::task::spawn_blocking(f),
    )
    .await
    {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(ApiError::Internal(format!("task panicked: {e}"))),
        Err(_) => Err(ApiError::Timeout),
    }
}
