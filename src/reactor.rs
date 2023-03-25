//! Reactor is the executor and manager of all actors,
//!
//! serves as
//! - Actor Runner Schedule and Execution
//! - Actor Register maintenance and update
//!

#[cfg(not(feature = "log"))]
use crate::log;
use crate::{queue::Queue, register::Register};
use alloc::{boxed::Box, vec::Vec};
use core::{
    future::Future as CoreFuture,
    hint,
    pin::Pin,
    sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering},
    task::{Context as CoreContext, Poll},
};

/// Object that [Reactor](Reactor) manipulate with
pub struct ReactorPair {
    inner: Pin<Box<dyn CoreFuture<Output = ()>>>,
    handle: ReactorHandle,
}

impl ReactorPair {
    /// create an instance
    pub fn new<F>(fut: F) -> Self
    where
        F: CoreFuture<Output = ()> + 'static,
    {
        ReactorPair {
            inner: Box::pin(fut),
            handle: ReactorHandle::new(),
        }
    }
}

static POLL_PENDING: AtomicBool = AtomicBool::new(false);
static POLL_BUDGET: AtomicUsize = AtomicUsize::new(3);

/// Future execution reach an phase
/// where needs some uncertain time
///
/// **NOTE** that
/// - polling the in-memory-vector-based
/// stream does not take uncertain time
/// - polling remote html page does
/// - polling a sleep-based future does
pub fn pending_polled() {
    POLL_PENDING.store(true, Ordering::Release);
}

/// Future execution reach an phase
/// where needs some uncertain time
///
/// **NOTE** that
/// - polling the in-memory-vector-based
/// stream does not take uncertain time
/// - polling remote html page does
/// - polling a sleep-based future does
pub fn is_pending_polled() -> bool {
    POLL_PENDING.load(Ordering::Acquire)
}

/// increase the poll budget to enable
/// more future wake up
pub fn inc_poll_budget(delta: usize) {
    POLL_BUDGET.fetch_add(delta, Ordering::Release);
}

/// decrease the poll budget when
/// wake up the context
#[allow(dead_code)]
fn dec_poll_budget(delta: usize) -> usize {
    POLL_BUDGET.fetch_sub(delta, Ordering::Release)
}

/// reset it
fn reset_poll_pending() {
    POLL_PENDING.store(false, Ordering::Release);
}

/// reset it
fn reset_poll_budget() {
    POLL_BUDGET.store(3, Ordering::Release);
}

static HANDLECOUNT: AtomicUsize = AtomicUsize::new(1);

/// handle of Reactor Pair
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReactorHandle(usize);

impl ReactorHandle {
    pub(crate) fn new() -> Self {
        Self(HANDLECOUNT.fetch_add(1, Ordering::Relaxed) as _)
    }
}

/// orders that Reactor execute
pub enum ReactingOrder {
    /// exit the program
    Exit(i32),
    /// Abort a future
    Abort(ReactorHandle),
    /// executing a future
    Execute(ReactorPair),
}

unsafe impl Send for ReactingOrder {}
unsafe impl Sync for ReactingOrder {}

static REACTORSEAL: AtomicBool = AtomicBool::new(false);
static REACTOREXIT: AtomicBool = AtomicBool::new(false);
static REACTOREXITCODE: AtomicIsize = AtomicIsize::new(0);
static REACTORCACHE: Queue<ReactingOrder> = Queue::new_null();
static QUEUENULLINIT: AtomicBool = AtomicBool::new(false);

/// static mutable Reactor where store
/// all Pairs
static mut REACTOR: Reactor = Reactor { inner: Vec::new() };

/// Executor and Manager of all actors
///
/// Schedule task for execution
/// and maintains the [Register](Register)
/// queue
pub struct Reactor {
    pub(crate) inner: Vec<ReactorPair>,
}

impl Reactor {
    /// get the reference of global
    /// static `REACTOR`
    pub fn new() -> &'static Self {
        // **Safety**: With `INIT` guard, we assure that
        // REACTOR is initialized and its inner
        // pointer points a valid Reactor
        // So it is safe to de-reference the
        // underlying pointer
        unsafe { &REACTOR }
    }

    /// push a pair to the cache queue
    pub fn push(future: ReactorPair) {
        if !QUEUENULLINIT.load(Ordering::Acquire) {
            REACTORCACHE.assume_init();
            QUEUENULLINIT.store(true, Ordering::Release);
        }
        REACTORCACHE.push(ReactingOrder::Execute(future));
    }

    /// drive all reactor's futures into completion
    ///
    // **Safety**: as the inner data is guarded with
    // `REACTORSEAL`, any mutable actions happens
    // will trigger the guardian locked
    pub async fn as_future() {
        unsafe { &mut REACTOR }.await
    }

    /// execute an ReactingOrder
    pub fn execute(order: ReactingOrder) {
        log::debug!("New order for execution");
        if !QUEUENULLINIT.load(Ordering::Acquire) {
            REACTORCACHE.assume_init();
            QUEUENULLINIT.store(true, Ordering::Release);
        }
        REACTORCACHE.push(order);
    }

    /// get the length of polling futures
    pub fn len() -> usize {
        unsafe { REACTOR.inner.len() }
    }

    /// process reactor cache
    /// - poll reacting orders from cache
    fn epoll(inner: &mut Vec<ReactorPair>) {
        while let Some(order) = unsafe { REACTORCACHE.pop() } {
            match order {
                ReactingOrder::Abort(handle) => {
                    log::debug!("Reactor abort future: {:?}", handle.0);
                    for ind in 0..inner.len() {
                        if inner[ind].handle == handle {
                            inner.swap_remove(ind);
                            break;
                        }
                    }
                }
                ReactingOrder::Execute(future) => inner.push(future),
                ReactingOrder::Exit(code) => {
                    log::warn!("Exit code: {} received, Exiting ...", code);
                    REACTOREXIT.store(true, Ordering::Relaxed);
                    REACTOREXITCODE.store(code as _, Ordering::Relaxed);
                }
            }
        }
    }
}

impl CoreFuture for Reactor {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut CoreContext<'_>) -> Poll<()> {
        let mut index = 0;
        let inner = &mut self.as_mut().inner;
        Self::epoll(inner);

        while index < inner.len() {
            // after each item gets polled, check state
            // - exit or not
            if REACTOREXIT.load(Ordering::Relaxed) {
                let code = REACTOREXITCODE.load(Ordering::Relaxed);
                log::warn!("Exit code: {} received, Exiting ...", code);
                // exit the program and
                // clear all futures
                inner.clear();
                break;
            }

            let pair = &mut inner[index];
            match Pin::new(&mut pair.inner).poll(cx) {
                Poll::Pending => {
                    // inner future is not ready
                    // and continue to the next item
                    index += 1;
                }
                Poll::Ready(()) => {
                    // inner future is ready
                    // remove it from queue
                    // and continue to the next item
                    while REACTORSEAL.load(Ordering::Acquire) {
                        hint::spin_loop();
                    }
                    REACTORSEAL.store(true, Ordering::Release);
                    inner.swap_remove(index);
                    REACTORSEAL.store(false, Ordering::Relaxed);
                    Register::update();
                }
            }

            Self::epoll(inner);
        }

        if inner.is_empty() {
            reset_poll_pending();
            reset_poll_budget();
            return Poll::Ready(());
        }
        drop(inner);

        #[cfg(all(
            not(feature = "no-std"),
            all(feature = "unstable", feature = "force-poll")
        ))]
        {
            #[cfg(feature = "wasm32")]
            force_poll_wasm32(cx);
            #[cfg(feature = "std")]
            force_poll_std(cx);
        }
        let _budget = dec_poll_budget(1);
        #[cfg(not(feature = "force-poll"))]
        if _budget > 0 {
            log::trace!("Wake future in reactor");
            cx.waker().wake_by_ref();
        } else {
            log::error!("Unfinished Future HANGING INDEFINITELY. Future is **NOT WAKED** after `Poll::Pending` returned");
        }
        Poll::Pending
    }
}

#[cfg(all(feature = "wasm32", feature = "unstable", feature = "force-poll"))]
#[allow(dead_code)]
fn force_poll_wasm32(cx: &mut CoreContext<'_>) {
    // set timeout then wake
    // will do that
    use crate::rt::wasm_timeout::set_timeout;
    use wasm_bindgen::{prelude::*, JsCast};

    use core::task::Waker;
    use wasm_bindgen::closure::Closure;

    static mut WAKER: Option<Waker> = None;
    unsafe {
        if WAKER.is_none() {
            WAKER.replace(cx.waker().clone());
        }
        // the future has been moved to another
        // environment, then update the waker
        if !WAKER.as_ref().unwrap().will_wake(cx.waker()) {
            WAKER.replace(cx.waker().clone());
        }
    }

    let f = Closure::<dyn Fn()>::new(|| unsafe {
        WAKER.as_ref().unwrap().wake_by_ref();
    })
    .into_js_value()
    .dyn_into::<js_sys::Function>()
    .expect("Closure to js function failed");
    let _ = set_timeout(&f, 50).unwrap_throw();
}

#[cfg(all(feature = "std", feature = "unstable", feature = "force-poll"))]
#[allow(dead_code)]
fn force_poll_std(cx: &mut CoreContext<'_>) {
    std::thread::sleep(std::time::Duration::from_millis(50));
    cx.waker().wake_by_ref();
}
