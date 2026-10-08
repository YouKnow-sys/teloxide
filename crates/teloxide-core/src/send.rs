//! Conditional `Send`/`Sync` helpers for the wasm32 target.
//!
//! On native targets these are plain [`Send`]/[`Sync`] bounds (and
//! [`MaybeSendBoxFuture`] is a `Send` boxed future). On `wasm32`, where futures are
//! driven by a single-threaded JS event loop and thus commonly aren't `Send`,
//! the bounds vanish, allowing `!Send` futures (e.g. reqwest's wasm backend) to
//! flow through the same APIs.

/// `Send` on native targets; no bound on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> MaybeSend for T {}

/// `Send` on native targets; no bound on `wasm32`.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}

/// `Sync` on native targets; no bound on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSync: Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Sync + ?Sized> MaybeSync for T {}

/// `Sync` on native targets; no bound on `wasm32`.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSync {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSync for T {}

/// A boxed future that is `Send` on native targets and not required to be
/// `Send` on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub type MaybeSendBoxFuture<'a, T> = futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
pub type MaybeSendBoxFuture<'a, T> = futures::future::LocalBoxFuture<'a, T>;

/// A boxed stream that is `Send` on native targets and not required to be
/// `Send` on `wasm32`.
#[cfg(not(target_arch = "wasm32"))]
pub type MaybeSendBoxStream<'a, T> = futures::stream::BoxStream<'a, T>;
#[cfg(target_arch = "wasm32")]
pub type MaybeSendBoxStream<'a, T> = futures::stream::LocalBoxStream<'a, T>;
