//! Cooperative harness cancellation; a second signal retains its normal force-exit behavior.

use std::cell::Cell;
use std::future::Future;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use futures_util::future::{Either, select};

use crate::{Error, Result};

static INTERRUPTED: OnceLock<Arc<AtomicBool>> = OnceLock::new();
thread_local! { static CLEANUP: Cell<Option<Instant>> = const { Cell::new(None) }; }

pub fn install() -> Result<()> {
    let flag = Arc::new(AtomicBool::new(false));
    for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
        signal_hook::flag::register_conditional_default(signal, Arc::clone(&flag))?;
        signal_hook::flag::register(signal, Arc::clone(&flag))?;
    }
    INTERRUPTED
        .set(flag)
        .map_err(|_| crate::invalid("signal handlers already installed"))?;
    Ok(())
}

pub fn requested() -> bool {
    INTERRUPTED
        .get()
        .is_some_and(|flag| flag.load(Ordering::Relaxed))
}

pub fn check() -> Result<()> {
    if requested() && CLEANUP.get().is_none() {
        Err(Error::Interrupted)
    } else {
        Ok(())
    }
}

/// Remaining shared diagnostics budget, including screenshots and nested collection.
pub fn budget(default: Duration) -> Duration {
    CLEANUP.get().map_or(default, |deadline| {
        default.min(deadline.saturating_duration_since(Instant::now()))
    })
}

pub struct Cleanup(Option<Instant>);

impl Cleanup {
    pub fn enter() -> Self {
        let previous = CLEANUP.get();
        CLEANUP.set(Some(
            previous.unwrap_or_else(|| Instant::now() + Duration::from_secs(30)),
        ));
        Self(previous)
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        CLEANUP.set(self.0);
    }
}

pub async fn interrupt<T>(future: impl Future<Output = T>) -> Result<T> {
    check()?;
    let cancelled = async {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if check().is_err() {
                return;
            }
        }
    };
    match select(Box::pin(future), Box::pin(cancelled)).await {
        Either::Left((value, _)) => Ok(value),
        Either::Right(_) => Err(Error::Interrupted),
    }
}
