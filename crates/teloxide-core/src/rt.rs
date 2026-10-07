//! Runtime primitives that work on both native targets and `wasm32`.
//!
//! On `wasm32-unknown-unknown`, `tokio::time` is unsupported and
//! [`std::time::Instant::now`] panics. This module provides sleeping, task
//! spawning and the current time behind one API, backed by [`tokio`] natively
//! and by [`gloo-timers`], [`wasm-bindgen-futures`] and [`web-time`] on
//! `wasm32`.
//!
//! Use these instead of tokio or std::time::Instant directly, so the code
//! stays portable.
//!
//! [`tokio`]: https://docs.rs/tokio
//! [`gloo-timers`]: https://docs.rs/gloo-timers
//! [`wasm-bindgen-futures`]: https://docs.rs/wasm-bindgen-futures
//! [`web-time`]: https://docs.rs/web-time

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::send::MaybeSend;

/// An opaque moment in time; `std::time::Instant` natively, `web_time::Instant`
/// on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;

/// A future returned by [`sleep`] and [`sleep_until`].
///
/// The future is `Unpin` on all platforms; dropping it cancels the timer.
#[must_use = "futures do nothing unless you .await or poll them"]
pub struct Sleep {
    inner: SleepInner,
}

#[cfg(not(target_arch = "wasm32"))]
struct SleepInner(Pin<Box<tokio::time::Sleep>>);
#[cfg(target_arch = "wasm32")]
struct SleepInner(gloo_timers::future::TimeoutFuture);

/// Waits until `duration` has elapsed.
pub fn sleep(duration: Duration) -> Sleep {
    #[cfg(not(target_arch = "wasm32"))]
    let inner = SleepInner(Box::pin(tokio::time::sleep(duration)));
    #[cfg(target_arch = "wasm32")]
    let inner = SleepInner(gloo_timers::future::sleep(duration));
    Sleep { inner }
}

/// Waits until `deadline`.
///
/// Note: on `wasm32` the remaining duration is computed once, at creation.
#[cfg(not(target_arch = "wasm32"))]
pub fn sleep_until(deadline: Instant) -> Sleep {
    Sleep { inner: SleepInner(Box::pin(tokio::time::sleep_until(deadline.into()))) }
}

/// Waits until `deadline`.
///
/// Note: on `wasm32` the remaining duration is computed once, at creation.
#[cfg(target_arch = "wasm32")]
pub fn sleep_until(deadline: Instant) -> Sleep {
    sleep(deadline.saturating_duration_since(Instant::now()))
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        #[cfg(not(target_arch = "wasm32"))]
        {
            this.inner.0.as_mut().poll(cx)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Pin::new(&mut this.inner.0).poll(cx)
        }
    }
}

/// A handle to a spawned task; `tokio::task::JoinHandle` natively.
///
/// On `wasm32` this wraps a oneshot channel fed by
/// [`wasm_bindgen_futures::spawn_local`]; awaiting it yields the task's output
/// (the error case can only happen if the spawned future panicked).
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct JoinHandle<T> {
    rx: tokio::sync::oneshot::Receiver<T>,
}
#[cfg(target_arch = "wasm32")]
impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().rx).poll(cx).map_err(|_| JoinError(()))
    }
}

/// The counterpart of `tokio::task::JoinError` on `wasm32`.
///
/// Never actually constructed unless a spawned task panics.
#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct JoinError(());

#[cfg(target_arch = "wasm32")]
impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("spawned task failed")
    }
}

#[cfg(target_arch = "wasm32")]
impl std::error::Error for JoinError {}

/// A join handle to a spawned task.
#[cfg(not(target_arch = "wasm32"))]
pub use tokio::task::JoinHandle;

/// Spawns a future: [`tokio::spawn`] natively,
/// `wasm_bindgen_futures::spawn_local` on `wasm32`.
pub fn spawn<F>(fut: F) -> JoinHandle<F::Output>
where
    F: Future + MaybeSend + 'static,
    F::Output: MaybeSend + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::spawn(fut)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            let out = fut.await;
            let _ = tx.send(out);
        });
        JoinHandle { rx }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn sleep_elapses() {
        const DELAY: Duration = Duration::from_millis(50);

        let before = Instant::now();
        sleep(DELAY).await;
        assert!(Instant::now().duration_since(before) >= DELAY);

        // `sleep_until` with a future deadline completes too. (Deadlines in
        // the past are handled by `saturating_duration_since` and aren't
        // tested here: `Instant - Duration` would risk underflow on wasm32,
        // where instants are relative to the JS time origin.)
        sleep_until(Instant::now() + DELAY).await;
    }

    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn spawn_joins() {
        let handle = spawn(async { 41 + 1 });
        // The wasm32 handle never fails (there are no abort semantics).
        assert_eq!(handle.await.unwrap(), 42);
    }
}
