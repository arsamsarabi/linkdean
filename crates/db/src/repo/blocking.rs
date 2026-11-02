use linkdean_core::{Error, Result};
use tokio::sync::Semaphore;

/// Password hashes running at once. Each takes 19 MiB and a full core for a moment.
const MAX_KDF: usize = 4;
static KDF: Semaphore = Semaphore::const_new(MAX_KDF);

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

/// `run_blocking` for password hashing, limited so a burst queues rather than exhausting memory.
pub async fn run_kdf<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let _permit = KDF
        .acquire()
        .await
        .map_err(|e| Error::msg(format!("kdf semaphore: {e}")))?;
    run_blocking(f).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn kdf_work_never_exceeds_the_limit() {
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let tasks = (0..MAX_KDF * 5).map(|_| {
            let (running, peak) = (running.clone(), peak.clone());
            tokio::spawn(run_kdf(move || {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(5));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok(())
            }))
        });
        for task in tasks.collect::<Vec<_>>() {
            task.await.unwrap().unwrap();
        }
        assert!(peak.load(Ordering::SeqCst) <= MAX_KDF);
    }
}
