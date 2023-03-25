//! delayed Execution Routine and Events
//!
//! Once combined with repeation, many
//! useful variants obtained:
//! - Delayed Deliver (original)
//! - Instant Deliver
//! - Repeat Deliver
//! - Loop Deliver

#[cfg(not(feature = "log"))]
use crate::log;
#[cfg(feature = "time")]
use crate::time::Timing;
use crate::{
    actor::{Actor, Future, Handle},
    context::Context,
    message::Message,
    reactor::{inc_poll_budget, pending_polled},
};
use alloc::boxed::Box;
#[cfg(feature = "time")]
use core::time::Duration;
use core::{
    any::Any,
    pin::Pin,
    ptr,
    task::{Context as CoreContext, Poll},
};

/// send a delayed message with condition
/// and get executed once the condition
/// satisfied.
///
pub struct Delayer<A: Actor> {
    inner: Option<A::Message>,
    /// `delay` serve as a indicator to show
    /// whether the message should be delivered or not
    /// it return a optional boolean to notify the `Delayer`
    /// that to repeat deliver the message or not
    /// - Some(1) : Deliver the message only ONCE
    /// - None : Repeatedly deliver the message without end
    /// - Some(number) : Deliver the message with `number` times
    //delay: Pin<Box<dyn Future<A, Output = Option<usize>>>>,
    delay: DelayIndicator<A>,
    handle: Handle,
    state: DelayerState,
}

/// Indicator to direct the delayer
/// at runtime,
/// - `Continue`: just continue the execution, no
///   intercept happens
/// - `Abort`: intercept the signal and
///   stop blocking immediately
/// - `Emit`: emit the signal notify actor
///   and execute immediately
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DelayingState {
    Abort,
    Continue,
    Emit,
}

/// Delayer State
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum DelayerState {
    Created,
    Started,
    Running,
    Aborted,
    Emitted,
    Finished,
}

/// the return type
/// when some condition satisfied
pub enum Indicator<M> {
    /// the return type
    /// is message
    Message(M),
    /// the return type
    /// is the indicator of
    /// message Repeation
    Repeation(Option<usize>),
}

pub(crate) struct DelayIndicator<A: Actor> {
    // make sure cx must be woken
    // if some progress is made
    inner: Pin<Box<dyn Future<A, Output = Indicator<A::Message>>>>,
    repeated: usize,
}

impl<A: Actor> DelayIndicator<A> {
    pub(crate) fn inc(&mut self) {
        self.repeated += 1;
    }

    pub(crate) fn new<F>(f: F) -> Self
    where
        F: FnMut(&mut A, &mut Context<A>, &mut CoreContext<'_>) -> Poll<Indicator<A::Message>>
            + 'static
            + Unpin,
    {
        Self {
            inner: Box::pin(f),
            repeated: 0,
        }
    }

    pub(crate) fn repeated(&self) -> usize {
        self.repeated
    }
}

/// An abstraction for Actor's delaying routine
///
pub trait Delaying<A: Actor> {
    /// called before the Delayer
    fn started(&mut self, _: &mut Context<A>) {}

    /// change the state of the Delayer to
    /// abort/emit/continue the Delayer when
    /// some condition satisfied
    ///
    /// Real-Time control or more elaborated
    /// execution could be achieved right here
    fn state(&mut self, _: &mut Context<A>) -> DelayingState {
        DelayingState::Continue
    }

    /// called before the Delayer get aborted
    fn aborted(&mut self, _: &mut Context<A>) {}

    /// called after the Delayer emitted in advance
    fn emitted(&mut self, _: &mut Context<A>) {}

    /// called after the Delayer finished
    fn finished(&mut self, _: &mut Context<A>) {}
}

impl<A: Actor> Delayer<A> {
    /// get the handle of the Delayer
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// get the state
    #[allow(dead_code)]
    pub fn state(&self) -> DelayerState {
        self.state
    }

    /// set the state
    #[allow(dead_code)]
    pub fn set_state(&mut self, state: DelayerState) {
        self.state = state;
    }

    /// deliver the message with specified `delay` duration
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can known
    /// the time
    ///
    /// NOTE that the delayed duration is NO LESS than
    /// `delay` you specified, and is within minor deviation
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub fn from_duration<T: Timing + 'static>(message: A::Message, delay: Duration) -> Self {
        let deadline = T::now() + delay;
        let delay = move |_: &mut A, _: &mut Context<A>, _: &mut CoreContext<'_>| {
            if T::now() > deadline {
                Poll::Ready(Indicator::Repeation(Some(1)))
            } else {
                Poll::Pending
            }
        };
        let indicator = DelayIndicator::new(delay);
        Self {
            inner: Some(message),
            delay: indicator,
            handle: Handle::new(),
            state: DelayerState::Created,
        }
    }

    /// instantly deliver the message to its destination,
    /// and it nearly takes no time
    /// and it can be used as `Messager`
    pub fn instant(message: A::Message) -> Self {
        let delay = DelayIndicator::new(
            |_: &mut A,
             _: &mut Context<A>,
             _: &mut CoreContext<'_>|
             -> Poll<Indicator<A::Message>> {
                Poll::Ready(Indicator::Repeation(Some(1)))
            },
        );
        Self {
            inner: Some(message),
            delay,
            handle: Handle::new(),
            state: DelayerState::Created,
        }
    }

    /// periodically deliver the message to its destination,
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can known
    /// the time
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub unsafe fn repeat<T: Timing + 'static>(
        message: A::Message,
        duration: Option<Duration>,
        repeats: Option<usize>,
    ) -> Self {
        let mut deadline = duration.map(|d| T::now() + d);
        let delay = DelayIndicator::new(
            move |_: &mut A,
                  _: &mut Context<A>,
                  _: &mut CoreContext<'_>|
                  -> Poll<Indicator<A::Message>> {
                match duration {
                    // if duration is none, then emit
                    // the message at once
                    None => Poll::Ready(Indicator::Repeation(repeats)),
                    // duration is specified,
                    // wait for the delay being completed
                    Some(dur) => {
                        if T::now() >= *deadline.as_ref().unwrap() {
                            *deadline.as_mut().unwrap() += dur;
                            Poll::Ready(Indicator::Repeation(repeats))
                        } else {
                            Poll::Pending
                        }
                    }
                }
            },
        );
        Self {
            inner: Some(message),
            delay,
            handle: Handle::new(),
            state: DelayerState::Created,
        }
    }

    /// use a function/Clousre as condition
    /// to deliver the message
    pub fn from_fn<F>(message: Option<A::Message>, delay: F) -> Self
    where
        F: FnMut(&mut A, &mut Context<A>, &mut CoreContext<'_>) -> Poll<Indicator<A::Message>>
            + 'static
            + Unpin,
    {
        Self {
            inner: message,
            delay: DelayIndicator::new(delay),
            handle: Handle::new(),
            state: DelayerState::Created,
        }
    }
}

// TODO Modify the poll routine
// after the actor is not alive
impl<A> Future<A> for Delayer<A>
where
    A: Actor + Delaying<A>,
    A::Message: Message,
    Self: Unpin,
{
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        act: &mut A,
        ctx: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<Self::Output> {
        match self.state {
            DelayerState::Created => {
                let state = <A as Delaying<A>>::state(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Created {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    DelayingState::Abort => {
                        // abort the Delayer
                        self.state = DelayerState::Aborted;
                        log::debug!("Delayer Aborted when Created");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    DelayingState::Emit => {
                        // emit the delayer in advance
                        self.state = DelayerState::Emitted;
                        log::debug!("Delayer Emitted when Created");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                self.state = DelayerState::Started;
                log::debug!("Delayer has successfully started");
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            DelayerState::Started => {
                let state = <A as Delaying<A>>::state(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Started {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    DelayingState::Abort => {
                        // abort the Delayer
                        self.state = DelayerState::Aborted;
                        log::debug!("Delayer Aborted when Created");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    DelayingState::Emit => {
                        // emit the delayer in advance
                        self.state = DelayerState::Emitted;
                        log::debug!("Delayer Emitted when Created");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                <A as Delaying<A>>::started(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Started {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Delayer has successfully started");
                self.state = DelayerState::Running;
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            DelayerState::Running => {
                let state = <A as Delaying<A>>::state(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Running {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    DelayingState::Continue => match self.delay.inner.as_mut().poll(act, ctx, cx) {
                        Poll::Pending => {
                            // TODO avoid too much waking
                            pending_polled();
                            Poll::Pending
                        }
                        Poll::Ready(indicator) => {
                            match indicator {
                                Indicator::Repeation(repeat) => {
                                    if self.inner.is_none() {
                                        log::error!("Message is None for execution");
                                        return Poll::Ready(());
                                    }
                                    match repeat {
                                        // run at least num times
                                        Some(num) => {
                                            // **Safety**: Just use the bitwise copy
                                            // of A::Message to avoid
                                            // memory safety violation
                                            let mut msg = unsafe {
                                                ptr::read_unaligned(
                                                    &self.inner as *const Option<A::Message>,
                                                )
                                            };
                                            A::action(act, msg.take().unwrap(), ctx);
                                            self.delay.inc();
                                            // the delayer state is changed
                                            // re-poll it
                                            if self.state != DelayerState::Running {
                                                log::debug!("Delayer state changed");
                                                cx.waker().wake_by_ref();
                                                inc_poll_budget(2);
                                                return Poll::Pending;
                                            }
                                            if self.delay.repeated() >= num {
                                                // finished
                                                self.state = DelayerState::Finished;
                                            }
                                        }
                                        // run repeatedly
                                        None => {
                                            // **Safety**: Just use the bitwise copy
                                            // of A::Message to avoid
                                            // memory safety violation
                                            let mut msg = unsafe {
                                                ptr::read_unaligned(
                                                    &self.inner as *const Option<A::Message>,
                                                )
                                            };
                                            A::action(act, msg.take().unwrap(), ctx);
                                            // the delayer state is changed
                                            // re-poll it
                                            if self.state != DelayerState::Running {
                                                log::debug!("Delayer state changed");
                                                cx.waker().wake_by_ref();
                                                inc_poll_budget(2);
                                                return Poll::Pending;
                                            }
                                        }
                                    }
                                }
                                // TODO the message is only
                                // available when the future is executed
                                // it can serve as a delayed future
                                // which sends a message
                                Indicator::Message(msg) => {
                                    // inner message will be ignored
                                    // though it is present
                                    A::action(act, msg, ctx);
                                    // the delayer state is changed
                                    // re-poll it
                                    if self.state != DelayerState::Running {
                                        log::debug!("Delayer state changed");
                                        cx.waker().wake_by_ref();
                                        inc_poll_budget(2);
                                        return Poll::Pending;
                                    }
                                    self.state = DelayerState::Finished;
                                }
                            }
                            // although it is probably finished,
                            // but `A::finished` is not called
                            cx.waker().wake_by_ref();
                            inc_poll_budget(2);
                            Poll::Pending
                        }
                    },
                    DelayingState::Abort => {
                        self.state = DelayerState::Aborted;
                        //log::debug!("Delayer is successfully Aborted");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        Poll::Pending
                    }
                    DelayingState::Emit => {
                        self.state = DelayerState::Emitted;
                        if self.inner.is_none() {
                            log::error!("Message is Empty to emit");
                            return Poll::Ready(());
                        }
                        A::action(act, self.inner.take().unwrap(), ctx);
                        // the delayer state is changed
                        // re-poll it
                        if self.state != DelayerState::Running {
                            log::debug!("Delayer state changed");
                            cx.waker().wake_by_ref();
                            inc_poll_budget(2);
                            return Poll::Pending;
                        }
                        //<A as Delaying<A>>::emitted(act, ctx);
                        //log::debug!("Delayer is successfully Emitted");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        Poll::Pending
                    }
                }
            }

            DelayerState::Aborted => {
                ctx.abort_future(self.handle);
                <A as Delaying<A>>::aborted(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Aborted {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Delayer is successfully Aborted");
                Poll::Ready(())
            }

            DelayerState::Emitted => {
                A::emitted(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Emitted {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                self.state = DelayerState::Finished;
                log::debug!("Delayer is successfully emitted in advance");
                Poll::Ready(())
            }

            DelayerState::Finished => {
                <A as Delaying<A>>::finished(act, ctx);
                // the delayer state is changed
                // re-poll it
                if self.state != DelayerState::Finished {
                    log::debug!("Delayer state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Delayer is successfully Finished");
                Poll::Ready(())
            }
        }
    }

    fn downcast_ref(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn downcast_mut(self: Pin<&mut Self>) -> Option<Pin<&mut dyn Any>> {
        Some(self)
    }
}
