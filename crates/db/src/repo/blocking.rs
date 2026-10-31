use linkdean_core::{Error, Result};

/// Run CPU-bound crypto/KDF off the async runtime.
pub async fn run_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::msg(format!("blocking task join: {e}")))?
}
