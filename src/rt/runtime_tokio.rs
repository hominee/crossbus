use super::{SpawnJoinHandle, Spawning};
#[cfg(not(feature = "log"))]
use crate::log;
use core::future::Future as CoreFuture;
use tokio::{
    runtime,
    task::{JoinError, JoinHandle, LocalSet},
};

impl<T: Send + Sync + Unpin> SpawnJoinHandle<Result<T, JoinError>> for JoinHandle<T> {}

/// impl a very bare tokio-based runtime
///
/// **NOTE** that it does not support features
/// `time` `io` of tokio. If you wanna
/// these features, build your own runtime
/// beforehand
pub struct Runtime;

/// the tokio runtime
pub struct TokioRuntime {
    pub(crate) rt: runtime::Runtime,
    pub(crate) local: LocalSet,
}

use core::{
    ptr::null_mut,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

static INIT: AtomicBool = AtomicBool::new(false);
const RT: *mut TokioRuntime = null_mut();
static RUNTIME: AtomicPtr<TokioRuntime> = AtomicPtr::new(RT);

impl TokioRuntime {
    pub fn new() -> &'static TokioRuntime {
        if !INIT.load(Ordering::Relaxed) {
            let rt_box = Box::new(Self {
                rt: runtime::Builder::new_current_thread().build().unwrap(),
                local: LocalSet::new(),
            });
            let raw_ptr = Box::into_raw(rt_box);
            RUNTIME.store(raw_ptr, Ordering::Relaxed);
            INIT.store(true, Ordering::Relaxed);
        }
        // **Safety**: With `INIT` guard, we assure that
        // RUNTIME is initialized and its inner
        // pointer points a valid TokioRuntime
        // So it is safe to de-reference the
        // underlying pointer
        unsafe { &*RUNTIME.load(Ordering::Relaxed) }
    }

    /// set new runtime with flush LocalSet
    pub fn set_runtime(rt: runtime::Runtime) {
        let rt_box = Box::new(Self {
            rt,
            local: LocalSet::new(),
        });
        let raw_ptr = Box::into_raw(rt_box);
        RUNTIME.swap(raw_ptr, Ordering::SeqCst);
        //let old_box = unsafe { Box::from_raw(old_ptr) };
        //RUNTIME.store(raw_ptr, Ordering::Release);
        INIT.store(true, Ordering::Relaxed);
    }
}

impl<T> Spawning<T, JoinHandle<T>, Result<T, JoinError>> for Runtime {
    fn spawn<F>(fut: F) -> JoinHandle<T>
    where
        F: CoreFuture<Output = T> + Send + 'static,
        F::Output: Send + 'static,
    {
        log::debug!("Spawn New future");
        let runner = TokioRuntime::new();
        runner.rt.spawn(fut)
    }

    fn spawn_blocking<F>(fut: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        log::debug!("Spawn blocking future");
        let runner = TokioRuntime::new();
        runner.rt.spawn_blocking(fut)
    }

    fn block_on<F>(fut: F) -> T
    where
        F: CoreFuture<Output = T>,
    {
        log::debug!("Blocking executing future");
        let runner = TokioRuntime::new();
        runner.rt.block_on(fut)
    }

    fn spawn_local<F>(fut: F) -> JoinHandle<T>
    where
        F: CoreFuture<Output = T> + 'static,
        F::Output: 'static,
    {
        log::debug!("Spawn local future");
        let runner = TokioRuntime::new();
        runner.local.spawn_local(fut)
    }
}
