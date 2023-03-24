use super::{SpawnJoinHandle, Spawning};
#[cfg(not(feature = "log"))]
use crate::log;
use core::future::Future as CoreFuture;
use core::future::{ready, Ready};

/// impl wasm-bindgen-futures-based runtime
pub struct Runtime;

impl SpawnJoinHandle<()> for Ready<()> {}

impl Spawning<(), Ready<()>, ()> for Runtime {
    fn spawn<F>(_: F) -> Ready<()>
    where
        F: CoreFuture<Output = ()> + Send + 'static,
    {
        unimplemented!("Wasm runtime only provides spawn_local for execution")
    }

    fn spawn_blocking<F>(_: F) -> Ready<()>
    where
        F: FnOnce() -> () + Send + 'static,
    {
        unimplemented!("Wasm runtime only provides spawn_local for execution")
    }

    fn block_on<F>(_: F)
    where
        F: CoreFuture<Output = ()>,
    {
        unimplemented!("Wasm runtime only provides spawn_local for execution")
    }

    fn spawn_local<F>(fut: F) -> Ready<()>
    where
        F: CoreFuture<Output = ()> + 'static,
    {
        log::debug!("Spawn local future");
        wasm_bindgen_futures::spawn_local(fut);
        ready(())
    }
}
