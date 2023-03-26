//! Convenient Runtime for common use
//!
//! it demonstrates the implementor how
//! to implement a runtime, may be removed
//! in the future or replace it with
//! a new crates or a document.
//!
//! Going Runtime less doesn't mean execution
//! without runtime, but more procisely,
//! **no built-in runtime, but allow any runtime,**
//!
//! it **DOES NOT** mean CrossBus runs without runtime.
//! on the contrary, CrossBus allow various runtimes and
//! customized runtime even a bare-bone executor.
//!
//! for convenience, CrossBus provides three common runtimes
//! - bare tokio-based runtime (with feature **tokio**),
//!   just a few crates, however, no `io`, `time` supported
//!   (build your own if necessary)
//! - async-std-based runtime (with feature **async-std**), kind of
//!   large and redundant
//! - wasm-bindgen-futures-based runtime (with feature **wasm32**),
//!   only `spawn_local` supported
//!
#[cfg(feature = "async-std")]
#[cfg_attr(docsrs, doc(cfg(feature = "async-std")))]
pub mod runtime_async_std;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod runtime_tokio;

#[cfg(feature = "wasm32")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm32")))]
pub mod runtime_wasm32;
#[cfg(feature = "wasm32")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm32")))]
pub mod wasm_timeout;

use core::future::Future as CoreFuture;

/// An abstraction for Actor's Runtime routine
///
/// **NOTE** that not all methods need to be implemented
/// it is okay to implement what you need to use and
/// leave rest methods unimplemented, check the implementation
/// of [wasm runtime](crate::rt::runtime_wasm32) for example.
pub trait Spawning<T, H, U> {
    fn spawn<F>(fut: F) -> H
    where
        F: CoreFuture<Output = T> + Send + 'static,
        T: Send + 'static,
        H: SpawnJoinHandle<U>;

    fn spawn_blocking<F>(fut: F) -> H
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
        H: SpawnJoinHandle<U>;

    fn block_on<F>(fut: F) -> T
    where
        F: CoreFuture<Output = T>;

    fn spawn_local<F>(fut: F) -> H
    where
        F: CoreFuture<Output = T> + 'static,
        T: 'static,
        H: SpawnJoinHandle<U>;
}

/// trait that the return type of [Spawning](Spawning) must implement
///
/// if `()` is returned, use [Ready](core::future::Ready) instead.
pub trait SpawnJoinHandle<U>: Send + Sync + Unpin + CoreFuture<Output = U> {}
