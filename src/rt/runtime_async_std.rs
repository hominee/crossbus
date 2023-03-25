use super::{SpawnJoinHandle, Spawning};
#[cfg(not(feature = "log"))]
use crate::log;
use async_std::task::{spawn_blocking, Builder, JoinHandle};
use core::future::Future as CoreFuture;

/// impl async-std-based runtime
pub struct Runtime;

impl<T: Send + Sync + Unpin> SpawnJoinHandle<T> for JoinHandle<T> {}

/// async-std has a global runtime
/// each time a future is spawned
/// the global runtime is used
impl<T> Spawning<T, JoinHandle<T>, T> for Runtime {
    fn spawn<F>(fut: F) -> JoinHandle<T>
    where
        F: CoreFuture<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        log::debug!("Spawn New future");
        Builder::new().spawn(fut).expect("Spawn task failed")
    }

    fn spawn_blocking<F>(fut: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        log::debug!("Spawn blocking future");
        spawn_blocking(fut)
    }

    fn block_on<F>(fut: F) -> T
    where
        F: CoreFuture<Output = T>,
    {
        log::debug!("Blocking executing future");
        Builder::new().blocking(fut)
        //block_on(fut)
    }

    fn spawn_local<F>(fut: F) -> JoinHandle<T>
    where
        F: CoreFuture<Output = T> + 'static,
        F::Output: 'static,
    {
        log::debug!("Spawn local future");
        Builder::new().local(fut).expect("Spawn task failed")
    }
}
