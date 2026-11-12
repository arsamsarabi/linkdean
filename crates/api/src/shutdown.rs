//! Graceful shutdown on SIGTERM or Ctrl-C.
//!
//! In the container the API runs as PID 1, and the kernel ignores signals PID 1 has no
//! handler for, so without this `docker stop` waited its full 10 s and then killed the
//! process mid-request.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// How long in-flight requests get to finish after the signal. Below Docker's default 10 s
/// stop timeout, so the process exits by itself before Docker kills it.
pub const DRAIN_CAP: Duration = Duration::from_secs(8);

/// Resolves on SIGTERM or Ctrl-C, then wakes `stopping` so the drain cap starts.
pub async fn signal(stopping: Arc<Notify>) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::warn!(error = %e, "cannot listen for SIGTERM");
                std::future::pending::<()>().await
            }
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
    tracing::info!("shutting down: no new connections, finishing open requests");
    stopping.notify_one();
}

/// Runs `server` to completion, but once `stopping` fires gives it at most `cap` to drain.
/// Returns `None` when the cap ran out with requests still open.
pub async fn drain_with_cap<F: Future>(
    server: F,
    stopping: Arc<Notify>,
    cap: Duration,
) -> Option<F::Output> {
    tokio::pin!(server);
    tokio::select! {
        out = &mut server => return Some(out),
        _ = stopping.notified() => {}
    }
    tokio::time::timeout(cap, server).await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn a_server_that_finishes_in_time_returns_its_result() {
        let stopping = Arc::new(Notify::new());
        stopping.notify_one();
        let server = async {
            tokio::time::sleep(DRAIN_CAP - Duration::from_millis(1)).await;
            7
        };
        assert_eq!(drain_with_cap(server, stopping, DRAIN_CAP).await, Some(7));
    }

    #[tokio::test(start_paused = true)]
    async fn a_stuck_server_is_abandoned_at_the_cap() {
        let stopping = Arc::new(Notify::new());
        stopping.notify_one();
        let start = tokio::time::Instant::now();
        let out = drain_with_cap(std::future::pending::<()>(), stopping, DRAIN_CAP).await;
        assert_eq!(out, None);
        assert_eq!(start.elapsed(), DRAIN_CAP);
    }

    #[tokio::test(start_paused = true)]
    async fn without_a_signal_the_server_runs_past_the_cap() {
        let stopping = Arc::new(Notify::new());
        let server = async {
            tokio::time::sleep(DRAIN_CAP * 10).await;
            1
        };
        assert_eq!(drain_with_cap(server, stopping, DRAIN_CAP).await, Some(1));
    }
}
