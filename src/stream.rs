//! Stream Execution Routine and Events
//!
#[cfg(not(feature = "log"))]
use crate::log;
use crate::{
    actor::{Actor, ActorState, Future, Handle},
    context::Context,
    reactor::{inc_poll_budget, pending_polled},
};
use core::{
    any::Any,
    marker::PhantomData,
    pin::Pin,
    task::{Context as CoreContext, Poll},
};
use futures_core::stream;
use pin_project_lite::pin_project;

/// Indicator to direct the Streaming
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
pub enum StreamingState {
    Abort,
    Continue,
    Pause,
    Resume,
}

/// Stream state
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum StreamState {
    Created,
    Started,
    Running,
    Aborted,
    Paused,
    Resumed,
    Finished,
}

/// An abstraction for Actor's stream routine
///
pub trait Stream<Item>
where
    Self: Actor,
{
    /// called before the actor emit the first Strem Item
    fn started(&mut self, _ctx: &mut Context<Self>) {}

    /// action to do for the Stream Item
    fn action(&mut self, msg: Item, ctx: &mut Context<Self>);

    /// change the state of stream
    /// to abort/pause/resume the stream accordingly
    ///
    /// Real-Time control or more elaborated
    /// execution could be achieved right here
    fn state(&mut self, _ctx: &mut Context<Self>) -> StreamingState {
        StreamingState::Continue
    }

    /// spawn stream to the actor
    fn spawn_stream<S>(&mut self, ctx: &mut Context<Self>, stream: S) -> Handle
    where
        S: stream::Stream + 'static,
        Self: Stream<S::Item>,
    {
        if ctx.state() == ActorState::Stopped {
            log::error!("Actor Stopped and Unable to add a stream");
            Handle::default()
        } else {
            ctx.spawn(Streaming::new(stream))
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

pin_project! {
    /// streaming a batch of items
    /// and send them to [`Stream::action`](crate::stream::Stream::action)
    pub struct Streaming<A: Actor, S: stream::Stream>
    {
        #[pin]
        stream: S,
        //timer: StreamTimer<A, S>,
        state: StreamState,
        handle: Handle,
        _marker: PhantomData::<A>,
    }
}

impl<A: Actor, S: stream::Stream> Streaming<A, S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            //timer: StreamTimer::new(),
            state: StreamState::Created,
            handle: Handle::new(),
            _marker: PhantomData::<A>,
        }
    }

    /// get the handle
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// get the state
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// set the state
    pub fn set_state(&mut self, state: StreamState) {
        self.state = state;
    }

    /*
     * /// it is unsafe since the inner field
     * /// is pinned. It is the caller's responsibility
     * /// to guarantee the safety to call it
     * ///
     * /// and may impl it otherwise to make it safe
     * /// make it pravite now
     *pub(crate) unsafe fn reset_inner(self: Pin<&mut Self>, stream: S) {
     *    unsafe {
     *        self.get_unchecked_mut().stream = stream;
     *    }
     *}
     */
}

/*
 *struct StreamTimer<A, S>
 *where
 *    S: stream::Stream,
 *    A: Actor,
 *{
 *    timer: Option<Duration>,
 *    item_timer: Option<Duration>,
 *    stream_by: Option<f64>,
 *    item_by: Option<f64>,
 *}
 */

/*
 *impl<A: Actor, S: stream::Stream> StreamTimer<A, S> {
 *    pub fn new() -> Self {
 *        Self {
 *            timer: None,
 *            item_timer: None,
 *            stream_by: None,
 *            item_by: None,
 *        }
 *    }
 *
 *    pub fn poll_next(
 *        self: Pin<&mut Self>,
 *        stream: Pin<&mut S>,
 *        timing: impl Timing,
 *        act: &mut A,
 *        ctx: &mut Context<A>,
 *        cx: &mut CoreContext<'_>,
 *    ) -> Poll<Option<S::Item>> {
 *        // set deadline if none
 *        if self.timer.is_some() {
 *            let now = timing.as_secs_f64() + self.timer.as_ref().unwrap().as_secs_f64();
 *            self.stream_by.replace(now);
 *        }
 *
 *        // check stream timeout or not
 *
 *        // check stream item timeout or not
 *
 *        //
 *    }
 *}
 */

// impl the poll of streaming
impl<A, S> Future<A> for Streaming<A, S>
where
    S: stream::Stream + 'static,
    A: Actor + Stream<S::Item>,
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
            StreamState::Created => {
                let state = <A as Stream<S::Item>>::state(act, ctx);
                // the Stream state is changed
                // re-poll it
                if *this.state != StreamState::Created {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    StreamingState::Abort => {
                        *this.state = StreamState::Aborted;
                        log::debug!("Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    StreamingState::Pause => {
                        *this.state = StreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                *this.state = StreamState::Started;
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                return Poll::Pending;
            }

            // stream started
            StreamState::Started => {
                let state = <A as Stream<S::Item>>::state(act, ctx);
                // the Stream state is changed
                // re-poll it
                if *this.state != StreamState::Started {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                match state {
                    StreamingState::Abort => {
                        *this.state = StreamState::Aborted;
                        log::debug!("Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    StreamingState::Pause => {
                        *this.state = StreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }
                <A as Stream<S::Item>>::started(act, ctx);
                // the stream state is changed
                // re-poll it
                if *this.state != StreamState::Started {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Stream has successfully started");
                *this.state = StreamState::Running;
                log::debug!("Actor Streaming");
                // make sure that stream handle exists
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                return Poll::Pending;
            }

            // stream is running
            StreamState::Running => {
                let state = <A as Stream<S::Item>>::state(act, ctx);
                // the Stream state is changed
                // re-poll it
                if *this.state != StreamState::Running {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                // to check if `abort` is called to stop the stream
                match state {
                    StreamingState::Abort => {
                        *this.state = StreamState::Aborted;
                        log::debug!("Stream is Aborting");
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    StreamingState::Pause => {
                        *this.state = StreamState::Paused;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        return Poll::Pending;
                    }
                    _ => {}
                }

                match this.stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(msg)) => {
                        <A as Stream<S::Item>>::action(act, msg, ctx);
                        // the stream state is changed
                        // re-poll it
                        if *this.state != StreamState::Running {
                            log::debug!("Stream state changed");
                            cx.waker().wake_by_ref();
                            inc_poll_budget(2);
                            return Poll::Pending;
                        }
                        Poll::Pending
                    }
                    Poll::Ready(None) => {
                        *this.state = StreamState::Finished;
                        cx.waker().wake_by_ref();
                        inc_poll_budget(2);
                        Poll::Pending
                    }
                    Poll::Pending => {
                        pending_polled();
                        Poll::Pending
                    }
                }
                //cx.waker().wake_by_ref();
            }

            // stream get aborted and called to abort the stream
            StreamState::Aborted => {
                ctx.abort_future(*this.handle);
                <A as Stream<S::Item>>::aborted(act, ctx);
                // the stream state is changed
                // re-poll it
                if *this.state != StreamState::Aborted {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Stream is successfully Aborted");
                Poll::Ready(())
            }

            StreamState::Paused => {
                <A as Stream<S::Item>>::paused(act, ctx);
                log::debug!("Stream is successfully Paused");
                let state = <A as Stream<S::Item>>::state(act, ctx);
                // the Stream state is changed
                // re-poll it
                if *this.state != StreamState::Paused {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                // to check if `abort` is called to stop the stream
                if let StreamingState::Resume = state {
                    *this.state = StreamState::Resumed;
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                Poll::Pending
            }

            StreamState::Resumed => {
                <A as Stream<S::Item>>::resumed(act, ctx);
                // the stream state is changed
                // re-poll it
                if *this.state != StreamState::Resumed {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Stream is successfully Resumed");
                *this.state = StreamState::Running;
                cx.waker().wake_by_ref();
                inc_poll_budget(2);
                Poll::Pending
            }

            StreamState::Finished => {
                <A as Stream<S::Item>>::finished(act, ctx);
                // the stream state is changed
                // re-poll it
                if *this.state != StreamState::Finished {
                    log::debug!("Stream state changed");
                    cx.waker().wake_by_ref();
                    inc_poll_budget(2);
                    return Poll::Pending;
                }
                log::debug!("Stream successfully finished");
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
