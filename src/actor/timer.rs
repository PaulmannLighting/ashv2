use std::future::{Future, pending};
use std::io::{self, ErrorKind};
use std::time::{Duration, Instant};

use futures_timer::Delay;
use tokio::select;

/// Waits for a monotonic deadline. An absent deadline never wakes the task.
pub async fn wait_until(deadline: Option<Instant>) {
    if let Some(deadline) = deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            Delay::new(remaining).await;
        }
    } else {
        pending::<()>().await;
    }
}

/// Polls an operation until it finishes or its monotonic timeout elapses.
pub async fn timeout<F>(duration: Duration, future: F) -> io::Result<F::Output>
where
    F: Future,
{
    let deadline = Instant::now() + duration;
    select! {
        biased;
        output = future => Ok(output),
        () = wait_until(Some(deadline)) => Err(ErrorKind::TimedOut.into()),
    }
}
