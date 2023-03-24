use core::{
    future::Future,
    pin::Pin,
    ptr,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

unsafe fn nop(_: *const ()) {}

unsafe fn nop_(_: *const ()) -> RawWaker {
    RawWaker::new(ptr::null(), &VTABLE)
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(nop_, nop, nop, nop);

/// use simple future executor
/// for complex usage, a more elaborated
/// executor is preferred
///
/// use whole cpu and block thread
/// until the future is ready.
pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    let raw_waker = unsafe { nop_(ptr::null()) };
    let waker = unsafe { Waker::from_raw(raw_waker) };
    let mut cx = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => {}
        }
    }
}
