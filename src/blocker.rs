//! blocking Execution Routine and Events
//!
//! **NOTE** that any new message sent to this
//! actor will be **REJECTED** when the actor
//! got any new Blcoker.

#[cfg(not(feature = "log"))]
use crate::log;
#[cfg(feature = "time")]
use crate::time::Timing;
use crate::{
    actor::{Actor, Future, Handle},
    context::Context,
    reactor::{inc_poll_budget, pending_polled},
};
use alloc::boxed::Box;
use core::{
    any::Any,
    pin::Pin,
    task::{Context as CoreContext, Poll},
};

/// Indicator to direct the blocker
/// at runtime,
/// - `Continue`: just continue the execution, no
///   intercept happens
/// - `Abort`: intercept the signal and
///   stop blocking immediately
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum BlockingState {
    /// continue as usual
    Continue,
    /// abort the blocker
    Abort,
}

/// Blocker state
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum BlockerState {
    Created,
    Started,
    Running,
    Aborted,
    Finished,
}

/// An abstraction for Actor's blocking routine
///
/// **NOTE** that any new message sent to this
/// actor will be **REJECTED** when the actor
/// got any new Blcoker.
///
/// Blocker is not recoverable
/// once gets aborted
pub trait Blocking<A>
where
    A: Actor,
{
    /// called before the blocker get started
    fn started(&mut self, _: &mut Context<A>) {}

    /// Runtime control of the `Blcoker`
    /// to abort the actor.
    ///
    /// Real-Time control or more elaborated
    /// execution could be achieved right here
    ///
    fn state(&mut self, _: &mut Context<A>) -> BlockingState {
        BlockingState::Continue
    }

    /// called before the blocker get aborted
    fn aborted(&mut self, _: &mut Context<A>) {}

    /// called after the blocker successfully finished
    fn finished(&mut self, _: &mut Context<A>) {}
}

/// block the actor (or make the actor waiting)
/// until some condition in `block` is satisfied
///
/// Once a new `Blocker` comes, it will block
/// the actor immediately and reject any incoming
/// messages.
pub struct Blocker<A> {
    // make sure cx must be woken
    // if some progress is made
    inner: Pin<Box<dyn Future<A, Output = ()>>>,
    handle: Handle,
    state: BlockerState,
}

impl<A: Actor> Blocker<A> {
    /// create an instance
    pub fn new<F>(fut: F) -> Self
    where
        F: Future<A, Output = ()> + 'static,
    {
        Self {
            inner: Box::pin(fut),
            handle: Handle::new(),
            state: BlockerState::Created,
        }
    }

    /// get the handle
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// get the state
    #[allow(dead_code)]
    pub fn state(&self) -> BlockerState {
        self.state
    }

    /// set the state
    #[allow(dead_code)]
    pub fn set_state(&mut self, state: BlockerState) {
        self.state = state;
    }

    /// block the actor with specified duration
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can know
    /// the time
    ///
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub fn from_duration<T: Timing + 'static>(dur: core::time::Duration) -> Self {
        let deadline = T::now() + dur;
        let inner = move |_: &mut A, _: &mut Context<A>, _: &mut CoreContext<'_>| {
            if T::now() > deadline {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        };
        Self {
            inner: Box::pin(inner),
            handle: Handle::new(),
            state: BlockerState::Created,
        }
    }
}

impl<A> Future<A> for Blocker<A>
where
    A: Actor + Blocking<A>,
{
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        act: &mut A,
        ctx: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<()> {
        match self.state {
            BlockerState::Created => {
                let state = <A as Blocking<A>>::state(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Created {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    BlockingState::Abort => {
                        // abort the actor
                        self.state = BlockerState::Aborted;
                        log::debug!("Blocker Aborted when Created");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                self.state = BlockerState::Started;
                log::debug!("Blocker has successfully Started");
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            BlockerState::Started => {
                let state = <A as Blocking<A>>::state(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Started {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    BlockingState::Abort => {
                        // abort the actor
                        self.state = BlockerState::Aborted;
                        log::debug!("Blocker Aborted when Started");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                self.state = BlockerState::Running;
                log::debug!("Blocker is Running");
                <A as Blocking<A>>::started(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Started {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            BlockerState::Running => {
                let state = <A as Blocking<A>>::state(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Running {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    BlockingState::Continue => match self.inner.as_mut().poll(act, ctx, cx) {
                        Poll::Pending => {
                            pending_polled();
                            Poll::Pending
                        }
                        Poll::Ready(_) => {
                            self.state = BlockerState::Finished;
                            // although it is finished,
                            // but `A::finished` is not called
                            cx.waker().wake_by_ref();
                            inc_poll_budget(2);
                            Poll::Pending
                        }
                    },
                    BlockingState::Abort => {
                        // abort the Blocker
                        self.state = BlockerState::Aborted;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        Poll::Pending
                    }
                }
            }

            BlockerState::Aborted => {
                ctx.abort_future(self.handle);
                <A as Blocking<A>>::aborted(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Aborted {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Blocker is successfully Aborted");
                Poll::Ready(())
            }

            BlockerState::Finished => {
                <A as Blocking<A>>::finished(act, ctx);
                // the Blocker state is changed
                // re-poll it
                if self.state != BlockerState::Finished {
                    log::debug!("Blocker state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Blcoker is successfully Finished");
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
