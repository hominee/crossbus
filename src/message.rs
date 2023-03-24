//! Message Stream Execution Routine and Events
//!
#[cfg(not(feature = "log"))]
use crate::log;
use crate::{
    actor::{Actor, ActorState, Future, Handle},
    context::Context,
    reactor::{inc_poll_budget, pending_polled},
};
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::{
    any::Any,
    pin::Pin,
    task::{Context as CoreContext, Poll},
};
use futures_core::stream::Stream as CoreStream;
use pin_project_lite::pin_project;

/// Message that exchange between actors
///
/// One thing to notice is that this empty
/// trait presents here is
/// - to make a difference between [`Stream<Item>`](crate::stream::Stream)
/// and [`MStream<Message>`](crate::message::MStream)
/// - for future usage
pub trait Message {}

macro_rules! impl_message {
    ($type:ty) => {
        impl Message for $type {}
    };
}

impl_message!(());
impl_message!(bool);
impl_message!(char);
impl_message!(i8);
impl_message!(i16);
impl_message!(i32);
impl_message!(i64);
impl_message!(i128);
impl_message!(isize);
impl_message!(u8);
impl_message!(u16);
impl_message!(u32);
impl_message!(u64);
impl_message!(u128);
impl_message!(usize);
impl<T: Message> Message for Box<T> {}
impl<T: Message> Message for Arc<T> {}
impl<T: Message> Message for Option<T> {}
impl<T: Message, E> Message for Result<T, E> {}

/// An abstraction for Actor's message stream routine
///
pub trait MStream<Item>
where
    Self: Actor,
{
    /// called before the actor emit the first Strem Item
    fn started(&mut self, _ctx: &mut Context<Self>) {}

    /// change the state of stream
    /// to abort/pause/resume the stream accordingly
    ///
    /// Real-Time control or more elaborated
    /// execution could be achieved right here
    fn state(&mut self, _ctx: &mut Context<Self>) -> MStreamingState {
        MStreamingState::Continue
    }

    /// add stream to the actor
    fn spawn_mstream<S>(&mut self, ctx: &mut Context<Self>, stream: S)
    where
        Self: Actor<Message = S::Item> + MStream<S::Item>,
        S: CoreStream + 'static,
        S::Item: Message,
    {
        if ctx.state() == ActorState::Stopped {
            log::error!("Actor Stopped and Unable to add a stream");
        } else {
            ctx.spawn(MStreaming::new(stream));
        }
    }

    /// called after the actor aborts the stream
    fn aborted(&mut self, _ctx: &mut Context<Self>) {}

    /// called after the actor pause the stream
    fn paused(&mut self, _ctx: &mut Context<Self>) {}

    /// called after the actor resume the stream
    fn resumed(&mut self, _ctx: &mut Context<Self>) {}

    /// called after the actor send the last Strem Item
    fn finished(&mut self, _ctx: &mut Context<Self>) {}
}

/// Indicator to direct the MStreaming
/// at runtime,
/// - `Continue`: just continue the execution, no
///   intercept happens
/// - `Abort`: intercept the signal and
///   stop blocking immediately
/// - `Pause`: pause the stream
///   and freeze immediately
/// - `Resume`: restore the stream back
///   to `Running`
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MStreamingState {
    Abort,
    Continue,
    Pause,
    Resume,
}

/// Message stream state
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MStreamState {
    Created,
    Started,
    Running,
    Aborted,
    Paused,
    Resumed,
    Finished,
}

pin_project! {
    /// streaming a batch of message
    /// and send them to [`Actor::action`](crate::actor::Actor::action)
    pub struct MStreaming<S: CoreStream> {
        #[pin]
        stream: S,
        handle: Handle,
        state: MStreamState,
    }
}

impl<S: CoreStream> MStreaming<S> {
    /// create an instance
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            handle: Handle::new(),
            state: MStreamState::Created,
        }
    }

    /// get the handle
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// get the state
    #[allow(dead_code)]
    pub fn state(&self) -> MStreamState {
        self.state
    }

    /// set the state
    #[allow(dead_code)]
    pub fn set_state(&mut self, state: MStreamState) {
        self.state = state;
    }
}

impl<A, S> Future<A> for MStreaming<S>
where
    A: Actor<Message = S::Item> + MStream<S::Item>,
    S: CoreStream + 'static,
    S::Item: Message,
{
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        act: &mut A,
        ctx: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<Self::Output> {
        let mut this = self.project();

        match *this.state {
            // new stream arrives
            MStreamState::Created => {
                let state = <A as MStream<S::Item>>::state(act, ctx);
                // the message Stream state is changed
                // re-poll it
                if *this.state != MStreamState::Created {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    MStreamingState::Abort => {
                        *this.state = MStreamState::Aborted;
                        log::debug!("Message Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    MStreamingState::Pause => {
                        *this.state = MStreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                *this.state = MStreamState::Started;
                log::debug!("Message Stream has successfully started");
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                return Poll::Pending;
            }

            // stream started
            MStreamState::Started => {
                let state = <A as MStream<S::Item>>::state(act, ctx);
                // the Message Stream state is changed
                // re-poll it
                if *this.state != MStreamState::Started {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    MStreamingState::Abort => {
                        *this.state = MStreamState::Aborted;
                        log::debug!("Message Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    MStreamingState::Pause => {
                        *this.state = MStreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                *this.state = MStreamState::Running;
                log::debug!("Actor Streaming Message");
                <A as MStream<S::Item>>::started(act, ctx);
                // make sure that stream handle exists
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                return Poll::Pending;
            }

            // stream is running
            MStreamState::Running => {
                let state = <A as MStream<S::Item>>::state(act, ctx);
                // the Message Stream state is changed
                // re-poll it
                if *this.state != MStreamState::Running {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    MStreamingState::Abort => {
                        *this.state = MStreamState::Aborted;
                        log::debug!("Message Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    MStreamingState::Pause => {
                        *this.state = MStreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }

                match this.stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(msg)) => {
                        <A as Actor>::action(act, msg, ctx);
                        // the stream state is changed
                        // re-poll it
                        if *this.state != MStreamState::Running {
                            log::debug!("Message Stream state changed");
                            cx.waker().wake_by_ref();
                            inc_poll_budget(2);
                            return Poll::Pending;
                        }
                        Poll::Pending
                    }
                    Poll::Ready(None) => {
                        *this.state = MStreamState::Finished;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        Poll::Pending
                    }
                    Poll::Pending => {
                        pending_polled();
                        Poll::Pending
                    }
                }
            }

            // stream get aborted and called to abort the stream
            MStreamState::Aborted => {
                ctx.abort_future(*this.handle);
                <A as MStream<S::Item>>::aborted(act, ctx);
                // the stream state is changed
                // re-poll it
                if *this.state != MStreamState::Aborted {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Message Stream is successfully Aborted");
                Poll::Ready(())
            }

            MStreamState::Paused => {
                <A as MStream<S::Item>>::paused(act, ctx);
                log::debug!("Message Stream is successfully Paused");
                let state = <A as MStream<S::Item>>::state(act, ctx);
                // the Message Stream state is changed
                // re-poll it
                if *this.state != MStreamState::Paused {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                // to check if `abort` is called to stop the stream
                if let MStreamingState::Resume = state {
                    *this.state = MStreamState::Resumed;
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                }
                Poll::Pending
            }

            MStreamState::Resumed => {
                <A as MStream<S::Item>>::resumed(act, ctx);
                // the message stream state is changed
                // re-poll it
                if *this.state != MStreamState::Resumed {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Message Stream is successfully Resumed");
                *this.state = MStreamState::Running;
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            MStreamState::Finished => {
                A::finished(act, ctx);
                // the message stream state is changed
                // re-poll it
                if *this.state != MStreamState::Finished {
                    log::debug!("Message Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Message Stream successfully finished");
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
