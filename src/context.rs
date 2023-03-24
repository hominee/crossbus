//! actor-level execution state and local environment
//! during runtime
//!
//! Context starts before actor get created, and dropped
//! after actor closed, its lifecycle covers actor's.
//!
//! It kind of serves as a back-end of the actor, and
//! almost all actor methods get called with Context.  
//!

#[cfg(not(feature = "log"))]
use crate::log;
#[cfg(feature = "time")]
use crate::time::Timing;
use crate::{
    actor::{ActingState, Actor, ActorId, ActorState, Future, Handle, Localizer},
    address::{Addr, Pack, QueueError, Receiver, Sender},
    blocker::{Blocker, Blocking},
    delayer::{Delayer, Delaying, Indicator},
    message::{MStream, MStreaming, Message},
    reactor::{inc_poll_budget, Reactor, ReactorPair},
    register::ActorGuard,
    stream::{Stream, Streaming},
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
#[cfg(feature = "time")]
use core::time::Duration;
use core::{
    any::Any,
    fmt,
    future::Future as CoreFuture,
    pin::Pin,
    ptr::NonNull,
    task::{Context as CoreContext, Poll},
};
use futures_core::stream::Stream as CoreStream;

struct Pair<A> {
    inner: Pin<Box<dyn Future<A, Output = ()>>>,
    handle: Handle,
}

impl<A> fmt::Debug for Pair<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pair")
            .field("inner", &"Pin<Box<Future<_>>>")
            .field("handle", &self.handle)
            .finish()
    }
}

/// execution state and local environment
/// for actor during runtime
pub struct Context<A: Actor> {
    /// the unique id of the actor
    /// created when the actor
    /// get registered
    id: ActorId,
    /// task queue
    inner: Vec<Pair<A>>,
    pack: Arc<Pack<A::Message>>,
    bloc: Vec<Pair<A>>,
    state: ActorState,
    abortion: Vec<Handle>,
    /// used to get access to the
    /// inner futures of ContextRunner
    ///
    /// **Safety**: ONLY the following
    /// fields are allowed to be modified
    /// - ContextRunner.inner
    /// - ContextRunner.bloc
    ///
    /// `ContextRunner.act` is okay
    /// for reference
    /// other field `ContextRunner.ctx`
    /// ARE NOT ALLOED to access even
    /// to get a shareable reference
    tunnel: Option<NonNull<ContextRunner<A>>>,
}

impl<A: Actor> fmt::Debug for Context<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context<_>")
            .field("id", &self.id)
            .field("inner", &self.inner)
            .field("pack", &self.pack)
            .field("bloc", &self.bloc)
            .field("state", &self.state)
            .field("abortion", &self.abortion)
            .field("tunnel", &self.tunnel)
            .finish()
    }
}

impl<A: Actor> Context<A> {
    /// create an instance of Context
    ///
    /// ```
    /// struct CrossBus {}
    /// impl Actor for CrossBus {
    /// ...
    /// }
    ///
    /// let ctx = Context::<CrossBus>::new();
    /// let addr = ctx.address();
    /// ```
    pub fn new() -> Self {
        let pack = Pack::new();

        Self {
            id: 0,
            pack,
            state: ActorState::Created,
            bloc: Vec::new(),
            inner: Vec::new(),
            abortion: Vec::new(),
            tunnel: None,
        }
    }

    /// get the address of the actor
    /// what can laterly get access
    /// to `Sender` or `Receiver`
    ///
    /// ```
    /// let addr = ctx.address();
    /// let sender = addr.clone().sender();
    /// let receiver = addr.receiver();
    /// ...
    /// ```
    pub fn address(&self) -> Addr<A::Message> {
        Addr::new(self.pack.clone())
    }

    /// get the id of the actor
    pub fn id(&self) -> ActorId {
        self.id
    }

    /// set the id of the actor
    pub(crate) fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    /// spawn a new future for execution
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// async fn run() {
    ///   // do work here
    /// }
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         // spawn the `run` for execution
    ///         let dur = core::time::Duration::from_secs(1);
    ///         let local = Localizer::new(run());
    ///         ctx.spawn(local);
    ///     }
    /// }
    /// ```
    pub fn spawn<F>(&mut self, f: F) -> Handle
    where
        F: Future<A, Output = ()> + 'static,
    {
        let handle = Handle::new();
        let pair = Pair {
            inner: Box::pin(f),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// block the actor until the
    /// spawned future is completed
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         // block the actor for 1 second
    ///         let dur = core::time::Duration::from_secs(1);
    ///         let sleep_fut = tokio::time::sleep(dur);
    ///         let local = Localizer::new(sleep_fut);
    ///         ctx.blocking(local);
    ///     }
    /// }
    /// ```
    pub fn blocking<F>(&mut self, f: F) -> Handle
    where
        F: Future<A, Output = ()> + 'static,
        A: Blocking<A>,
    {
        let target = Blocker::new(f);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.bloc.push(pair);
        handle
    }

    /// block the actor with specified
    /// duration
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can known
    /// the time
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         // block the actor for 1 second
    ///         let dur = core::time::Duration::from_secs(1);
    ///         ctx.blocking_duration::<std::time::Instant>(dur);
    ///     }
    /// }
    /// ```
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub fn blocking_duration<T>(&mut self, duration: Duration) -> Handle
    where
        A: Blocking<A>,
        T: Timing + 'static,
    {
        let target = Blocker::from_duration::<T>(duration);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.bloc.push(pair);
        handle
    }

    /// send message to the queue
    /// messages may be rejected when actor is
    /// blocked or message queue is full/closed
    ///
    /// ```no_run rust
    /// struct Num(uisze);
    /// impl Message for Num {}
    ///
    /// struct CrossBus{
    ///     sum: isize
    /// }
    /// impl Actor for CrossBus {
    ///     type Message = Num;
    ///
    ///     fn create(ctx: &mut Context<Self>) -> Self {
    ///         Self { sum: 0, }
    ///     }
    ///
    ///     fn started(&mut self, ctx: &mut Context<Self>) {
    ///         ctx.send_message(Num(1));
    ///         ctx.send_message(Num(1));
    ///         ctx.send_message(Num(1));
    ///     }
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         self.sum += msg.0;
    ///     }
    /// }
    ///
    ///
    pub fn send_message(&mut self, msg: A::Message) -> Result<(), QueueError<A::Message>>
    where
        A::Message: Message + Send + 'static,
    {
        self.sender().send(msg)
    }

    /// send a batch of messages to the queue
    /// messages may be rejected when actor is
    /// blocked or message queue is full/closed
    ///
    /// ```no_run rust
    /// ...
    ///     fn started(&mut self, ctx: &mut Context<Self>) {
    ///         ctx.send_message_batch(vec![Num(1), Num(1), Num(1)]);
    ///     }
    /// ...
    /// ```
    pub fn send_message_batch(
        &mut self,
        msgs: Vec<A::Message>,
    ) -> Vec<Result<(), QueueError<A::Message>>>
    where
        A::Message: Message + Send + 'static,
    {
        msgs.into_iter().map(|msg| self.send_message(msg)).collect()
    }

    /// send and execute normal [future](`CoreFuture`)
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// // returns unit type `()`
    /// async fn run() {
    ///     // do work here
    /// }
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         let dur = core::time::Duration::from_secs(1);
    ///         ctx.send_future(run());
    ///     }
    /// }
    /// ```
    pub fn send_future<F>(&mut self, fut: F) -> Handle
    where
        F: CoreFuture<Output = ()> + 'static,
        A::Message: Message + 'static,
    {
        let local = Localizer::new(fut);
        let handle = Handle::new();
        let pair = Pair {
            inner: Box::pin(local),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// instantly deliver a message
    /// NOTE that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    ///
    /// ```no_run rust
    /// ...
    ///     fn started(&mut self, ctx: &mut Context<Self>) {
    ///         ctx.instant_message(Num(1));
    ///     }
    /// ...
    /// ```
    pub fn instant_message(&mut self, msg: A::Message) -> Handle
    where
        A::Message: Message + Unpin + 'static,
        A: Delaying<A>,
    {
        let target = Delayer::instant(msg);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// deliver a message with specified duration delay
    /// NOTE that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can known
    /// the time
    ///
    /// ```no_run rust
    /// ...
    ///     fn started(&mut self, ctx: &mut Context<Self>) {
    ///         let dur = core::time::Duration::from_secs(1);
    ///         ctx.delay_message(Num(1), dur);
    ///     }
    /// ...
    /// ```
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub fn delay_message<T: Timing + 'static>(&mut self, msg: A::Message, delay: Duration) -> Handle
    where
        A::Message: Message + Unpin + 'static,
        A: Delaying<A>,
    {
        let target = Delayer::from_duration::<T>(msg, delay);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// deliver a message with specified function that
    /// it will be delayed as long as the `f` is not
    /// Poll::Ready(_)
    /// NOTE that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    pub fn delay_message_fn<F>(&mut self, msg: A::Message, f: F) -> Handle
    where
        A::Message: Message + Unpin + 'static,
        A: Delaying<A>,
        F: FnMut(&mut A, &mut Context<A>, &mut CoreContext<'_>) -> Poll<Indicator<A::Message>>
            + 'static
            + Unpin,
    {
        let target = Delayer::from_fn(Some(msg), f);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// repeatedly deliver a message with specified times
    /// and interval duration
    ///
    /// feature **`time`** must be enabled
    /// and the [Timing](`Timing`) implementation
    /// is required with which the actor can known
    /// the time
    ///
    /// **Safety**: Since consuming `message` couple times
    /// involving memory safety, message mutating is **NOT**
    /// allowed after `repeat_message` consuming this `message`,
    /// eg it's ILLEGAL to mutate it in `Actor::action` or
    /// somewhere else which lead to unexpected behavior.
    /// It is caller's responsibility to prevent these
    ///
    /// **NOTE** that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    /// the argument `repeats` is `usize`, has three scenarios:
    /// - None: Infinitely deliver the message
    /// - Some(0): Just deliver the message **Once**
    /// - Some(number): Deliver the message `number` times
    ///
    /// ```no_run rust
    /// ...
    ///     fn started(&mut self, ctx: &mut Context<Self>) {
    ///         // repeat 3 times send message with 1s interval
    ///         let dur = core::time::Duration::from_secs(1);
    ///         ctx.repeat_message(Num(1), Some(dur), Some(3));
    ///     }
    /// ...
    /// ```
    #[cfg(feature = "time")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time")))]
    pub unsafe fn repeat_message<T: Timing + 'static>(
        &mut self,
        message: A::Message,
        dur: Option<Duration>,
        repeats: Option<usize>,
    ) -> Handle
    where
        A::Message: Message + Unpin + 'static,
        A: Delaying<A>,
    {
        let target = unsafe { Delayer::repeat::<T>(message, dur, repeats) };
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// deliver a message with specified
    /// function that it will be delayed
    /// as long as the `f` is not Poll::Ready(_)
    /// the function's Output **MUST** be `Indicator::Message`
    /// if not, it will log out the error,
    /// and gets ignored
    ///
    /// NOTE that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    pub fn delay_fn<F>(&mut self, f: F) -> Handle
    where
        A::Message: Message + Unpin + 'static,
        A: Delaying<A>,
        F: FnMut(&mut A, &mut Context<A>, &mut CoreContext<'_>) -> Poll<Indicator<A::Message>>
            + 'static
            + Unpin,
    {
        let target = Delayer::from_fn(None, f);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// spawn a stream into the actor
    ///
    ///  **NOTE** that the stream item
    /// will be processed by [`Stream::action`](`Stream::action`)
    /// **NOT** [`Actor::action`](`Actor::action`)
    ///
    /// **NOTE** that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// struct St { items: Vec<i32> }
    /// impl Stream for St {
    ///     // impl stream for St here
    ///     ...
    /// }
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         // spawn stream
    ///         let st = St {items: vec![1, 1, 1]};
    ///         ctx.streaming(st);
    ///     }
    /// }
    /// ```
    pub fn streaming<S>(&mut self, s: S) -> Handle
    where
        S: CoreStream + 'static,
        A: Stream<S::Item>,
    {
        let target = Streaming::new(s);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// spawn a message stream into the actor
    ///
    ///  **NOTE** that the stream item
    /// will be processed by [`Stream::action`](`Stream::action`)
    /// **NOT** [`Actor::action`](`Actor::action`)
    ///
    /// **NOTE** that the message bypass the message queue
    /// and get handled by `Actor::update` directly
    ///
    /// ``` no_run rust
    /// use crossbus::prelude::*;
    ///
    /// struct St { items: Vec<i32> }
    /// impl Stream for St {
    ///     // impl stream for St here
    ///     ...
    /// }
    ///
    /// impl Actor for CrossBus {
    ///
    ///     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
    ///         // spawn stream
    ///         let st = St {items: vec![Num(1), Num(1), Num(1)]};
    ///         ctx.streaming(st);
    ///     }
    /// }
    /// ```
    pub fn streaming_message<S>(&mut self, s: S) -> Handle
    where
        A: Actor<Message = S::Item> + MStream<S::Item>,
        S: CoreStream + 'static,
        S::Item: Message,
    {
        let target = MStreaming::new(s);
        let handle = target.handle();
        let pair = Pair {
            inner: Box::pin(target),
            handle,
        };
        self.inner.push(pair);
        handle
    }

    /// get the sender of the actor
    pub fn sender(&self) -> Sender<A::Message> {
        self.pack.inc_sender(1);
        self.pack.set_alive();
        inc_poll_budget(2);
        Sender {
            pack: self.pack.clone(),
        }
    }

    /// get the receiver of the actor
    pub fn receiver(&self) -> Receiver<A::Message> {
        Receiver {
            pack: self.pack.clone(),
        }
    }

    /// TODO: maybe impl it with `tunnel`
    /// mark the future the handle points to
    /// is to be aborted
    pub fn abort_future(&mut self, handle: Handle) {
        self.abortion.push(handle);
    }

    /// get the current state of the actor
    pub fn state(&self) -> ActorState {
        self.state.clone()
    }

    /// set the state of the actor
    /// be CAREFUL with it that
    /// it will change actor execution
    pub fn set_state(&mut self, state: ActorState) {
        self.state = state;
    }

    /// whether the actor is started or not
    pub fn is_started(&self) -> bool {
        self.state.is_started()
    }

    /// whether the actor is running state
    pub fn is_running(&self) -> bool {
        self.state == ActorState::Running
    }

    /// run the actor
    pub fn run(self, act: ActorGuard<A>) -> Addr<A::Message> {
        let addr = self.address();
        let runner = ContextRunner::new(self, act);
        Reactor::push(ReactorPair::new(runner));
        addr
    }

    /// whether the actor is in stopping or not
    pub fn is_stopping(&self) -> bool {
        self.state == ActorState::Stopping
    }

    /// whether the actor is stopped or not
    pub fn is_stopped(&self) -> bool {
        self.state == ActorState::Stopped
    }

    /// restart the context, it will
    /// - flush all blockers
    /// - flush all running tasks
    /// but the message queue will survive
    pub fn restart(&mut self) {
        self.state = ActorState::Started;
        self.inner.clear();
        self.bloc.clear();
        self.abortion.clear();
    }

    /// stop the context and the actor
    /// it can be restored if `Actor::state`
    /// returns `ActingState::Resume`
    /// if not, the actor will be stopped
    pub fn stop(&mut self) {
        self.state = ActorState::Stopping;
    }

    /// downcast the inner future into
    /// type `&T`, where `T` is  one
    /// of the following four types:
    /// - [stream::Streaming](crate::stream::Streaming)
    /// - [message::MStreaming](crate::message::MStreaming)
    /// - [blocker::Blocker](crate::blocker::Blocker)
    /// - [delayer::Delayer](crate::delayer::Delayer)
    pub fn downcast_ref<T: 'static>(&self, handle: Handle) -> Option<&T> {
        let runner: &ContextRunner<A> = (&self.tunnel)
            // **Safety**: the inner data is guaranted
            // to be not null
            .and_then(|boxed| Some(unsafe { boxed.as_ref() }))
            .unwrap();

        for pair in runner.inner.iter() {
            if pair.handle == handle {
                return pair
                    .inner
                    .downcast_ref()
                    .and_then(|any_| any_.downcast_ref::<T>());
            }
        }
        for pair in runner.bloc.iter() {
            if pair.handle == handle {
                return pair
                    .inner
                    .downcast_ref()
                    .and_then(|any_| any_.downcast_ref::<T>());
            }
        }
        None
    }

    /// downcast the inner future into
    /// type `&mut T`, where `T` is  one
    /// of the following four types:
    /// - [stream::Streaming](crate::stream::Streaming)
    /// - [message::MStreaming](crate::message::MStreaming)
    /// - [blocker::Blocker](crate::blocker::Blocker)
    /// - [delayer::Delayer](crate::delayer::Delayer)
    pub fn downcast_mut<T: 'static>(&mut self, handle: Handle) -> Option<Pin<&mut T>> {
        let runner: &mut ContextRunner<A> = (&mut self.tunnel)
            // **Safety**: the inner data is guaranted
            // to be not null
            .and_then(|mut boxed| Some(unsafe { boxed.as_mut() }))
            .unwrap();
        for pair in runner.inner.iter_mut() {
            if pair.handle == handle {
                let pin_dyn = (&mut pair.inner).as_mut().downcast_mut();
                let pin_data = pin_dyn
                    .and_then(|en: Pin<&mut dyn Any>| -> Option<&mut T> {
                        // **Safety**: Since the inner data is guarded
                        // with `Pin<Box<_>>` So
                        // We can guarantee that the data will
                        // never be moved out via the mutable
                        // reference
                        unsafe { en.get_unchecked_mut() }.downcast_mut::<T>()
                    })
                    // **Safety**: the inner data is pinned
                    // So it is safe to create a new Pin
                    .and_then(|en| Some(unsafe { Pin::new_unchecked(en) }));
                return pin_data;
            }
        }
        for pair in runner.bloc.iter_mut() {
            if pair.handle == handle {
                let pin_dyn = (&mut pair.inner).as_mut().downcast_mut();
                let pin_data = pin_dyn
                    .and_then(|en: Pin<&mut dyn Any>| -> Option<&mut T> {
                        // **Safety**: Since the inner data is guarded
                        // with `Pin<Box<_>>` So
                        // We can guarantee that the data will
                        // never be moved out via the mutable
                        // reference
                        unsafe { en.get_unchecked_mut() }.downcast_mut::<T>()
                    })
                    // **Safety**: the inner data is pinned
                    // So it is safe to create a new Pin
                    .and_then(|en| Some(unsafe { Pin::new_unchecked(en) }));
                return pin_data;
            }
        }
        for pair in self.inner.iter_mut() {
            if pair.handle == handle {
                let pin_dyn = (&mut pair.inner).as_mut().downcast_mut();
                let pin_data = pin_dyn
                    .and_then(|en: Pin<&mut dyn Any>| -> Option<&mut T> {
                        // **Safety**: Since the inner data is guarded
                        // with `Pin<Box<_>>` So
                        // We can guarantee that the data will
                        // never be moved out via the mutable
                        // reference
                        unsafe { en.get_unchecked_mut() }.downcast_mut::<T>()
                    })
                    // **Safety**: the inner data is pinned
                    // So it is safe to create a new Pin
                    .and_then(|en| Some(unsafe { Pin::new_unchecked(en) }));
                return pin_data;
            }
        }
        for pair in self.bloc.iter_mut() {
            if pair.handle == handle {
                let pin_dyn = (&mut pair.inner).as_mut().downcast_mut();
                let pin_data = pin_dyn
                    .and_then(|en: Pin<&mut dyn Any>| -> Option<&mut T> {
                        // **Safety**: Since the inner data is guarded
                        // with `Pin<Box<_>>` So
                        // We can guarantee that the data will
                        // never be moved out via the mutable
                        // reference
                        unsafe { en.get_unchecked_mut() }.downcast_mut::<T>()
                    })
                    // **Safety**: the inner data is pinned
                    // So it is safe to create a new Pin
                    .and_then(|en| Some(unsafe { Pin::new_unchecked(en) }));
                return pin_data;
            }
        }
        None
    }
}

/// Future-oriented runner that drive all tasks into
/// completion
pub struct ContextRunner<A>
where
    A: Actor,
{
    act: ActorGuard<A>,
    inner: Vec<Pair<A>>,
    bloc: Vec<Pair<A>>,
    ctx: Context<A>,
}

impl<A: Actor> fmt::Debug for ContextRunner<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextRunner<_>")
            .field("act", &self.act)
            .field("inner", &self.inner)
            .field("bloc", &self.bloc)
            .field("ctx", &self.ctx)
            .finish()
    }
}

impl<A: Actor> ContextRunner<A> {
    /// create an instance
    pub fn new(ctx: Context<A>, act: ActorGuard<A>) -> Self {
        let mut runner = Self {
            ctx,
            act,
            inner: Vec::new(),
            bloc: Vec::new(),
        };
        runner.tunnel();
        runner
    }

    /// **Safety**: **MUST** update the tunnel
    /// when new items (inner future / bloc)
    /// comes.
    /// when the underlying data the
    /// pointer refer to is changed
    /// and downcasting the items
    /// is **INVALID** and will panic
    /// the program
    pub(crate) fn tunnel(&mut self) {
        self.ctx.tunnel = NonNull::new(self);
    }

    /// pull new actor futures from context
    pub fn update(&mut self) -> bool {
        let mut blocked = false;
        let mut tunnel = false;
        if !self.ctx.bloc.is_empty() {
            blocked = true;
            tunnel = true;
            self.bloc.extend(self.ctx.bloc.drain(0..));
            log::info!("move bloc future into runner, {}", self.bloc.len());
        }
        if !self.ctx.inner.is_empty() {
            tunnel = true;
            self.inner.extend(self.ctx.inner.drain(0..));
            log::info!("move inner future into runner: {}", self.inner.len());
        }
        if !self.ctx.abortion.is_empty() {
            tunnel = true;
            'outer: while let Some(handle) = self.ctx.abortion.pop() {
                // remove spawned handle in ContextRunner
                for index in 0..self.inner.len() {
                    if self.inner[index].handle == handle {
                        self.inner.swap_remove(index);
                        continue 'outer;
                    }
                }

                // remove spawned handle in Context
                for index in 0..self.ctx.inner.len() {
                    if self.ctx.inner[index].handle == handle {
                        self.ctx.inner.swap_remove(index);
                        continue 'outer;
                    }
                }
            }
        }
        //self.ctx.pack.update_state();
        if tunnel {
            self.tunnel();
        }
        blocked
    }

    pub(crate) fn to_stop(&self) -> bool {
        match self.ctx.state {
            ActorState::Stopped | ActorState::Stopping => true,
            _ => false,
        }
    }

    /// should the runner continue to be
    /// alive or exit
    pub fn is_alive(&self) -> bool {
        if self.ctx.is_stopped() {
            return false;
        }
        self.ctx.pack.update_state();
        self.ctx.pack.sender_number() != 0
            || self.ctx.pack.is_alive()
            || self.ctx.pack.is_closing()
            || !self.ctx.state.is_started()
            || !self.inner.is_empty()
            || !self.bloc.is_empty()
    }

    /// if the actor is blocked or not
    pub fn is_blocked(&self) -> bool {
        !self.bloc.is_empty()
    }
}

unsafe impl<A: Actor> Send for ContextRunner<A> {}
unsafe impl<A: Actor> Sync for ContextRunner<A> {}

// NOTE Modify the poll routine
// after the actor is not alive
impl<A> CoreFuture for ContextRunner<A>
where
    A: Actor,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut CoreContext<'_>) -> Poll<()> {
        let this = self.get_mut();

        match this.ctx.state {
            // first poll
            ActorState::Created => {
                // even if the actor state is changed
                // after executing it
                // it is ignored though anyway
                // the actor should step into Started state
                // for this state change
                A::initial(&mut this.act, &mut this.ctx);
                log::debug!("Actor successfully created");
                match A::state(&mut this.act, &mut this.ctx) {
                    ActingState::Stop => {
                        // stop the actor
                        this.ctx.state = ActorState::Stopping;
                        log::debug!("Actor Stopping when Created");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    ActingState::Abort => {
                        // abort the actor
                        this.ctx.state = ActorState::Aborted;
                        log::debug!("Actor Aborted when Created");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    _ => {}
                }
                // if actor gets a new blocker here
                // the actor won't be blocked anyway
                // the actor should step into Running state
                // for this blocker
                this.update();
                this.ctx.state = ActorState::Started;
                log::debug!("Actor successfully started");
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            // actor has been started
            ActorState::Started => {
                match A::state(&mut this.act, &mut this.ctx) {
                    ActingState::Stop => {
                        // stop the actor
                        this.ctx.state = ActorState::Stopping;
                        log::debug!("Actor stopping when Started");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    ActingState::Abort => {
                        // abort the actor
                        this.ctx.state = ActorState::Aborted;
                        log::debug!("Actor Aborted when Started");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    _ => {}
                }
                A::started(&mut this.act, &mut this.ctx);
                // the actor state is changed
                // re-poll it
                if this.ctx.state != ActorState::Started {
                    log::debug!("Actor state changed");
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                // if actor gets a new blocker here
                // the actor won't be blocked anyway
                // the actor should step into Running state
                // for this blocker
                this.update();
                this.ctx.state = ActorState::Running;
                log::debug!("Actor successfully Running");
                cx.waker().wake_by_ref();
                Poll::Pending
            }

            // actor is running
            // this is the core part of actor future
            // that drive all operations into completion
            ActorState::Running => {
                match A::state(&mut this.act, &mut this.ctx) {
                    ActingState::Stop => {
                        // stop the actor
                        this.ctx.state = ActorState::Stopping;
                        log::debug!("Actor Stopping when to Running");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    ActingState::Abort => {
                        // abort the actor
                        this.ctx.state = ActorState::Aborted;
                        log::debug!("Actor Aborted when Running");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    _ => {}
                }
                // running loop for execution
                'ex: loop {
                    // if blocker exists in the context
                    // the actor is blocked
                    // the make the computation consistent
                    // the most recent blcoker has higher priority
                    while this.is_blocked() {
                        log::debug!("Actor blocked");
                        let bloc = this.bloc.last_mut().unwrap();
                        match bloc.inner.as_mut().poll(&mut this.act, &mut this.ctx, cx) {
                            Poll::Pending => {
                                //cx.waker().wake_by_ref();
                                return Poll::Pending;
                            }
                            Poll::Ready(()) => {
                                this.bloc.pop();
                                // the actor state is changed
                                // re-poll it
                                if this.ctx.state != ActorState::Running {
                                    log::debug!("Actor state changed");
                                    cx.waker().wake_by_ref();
                                    return Poll::Pending;
                                }
                                // update the incoming items
                                if this.update() {
                                    continue 'ex;
                                }
                            }
                        }
                    }

                    // pull messages
                    match this.ctx.pack.poll_next(cx) {
                        Poll::Pending => {}
                        Poll::Ready(Some(msg)) => {
                            log::debug!("New message");
                            // new message has been pulled
                            // use it
                            A::action(&mut this.act, msg, &mut this.ctx);
                            // the actor state is changed
                            // re-poll it
                            if this.ctx.state != ActorState::Running {
                                log::debug!("Actor state changed");
                                cx.waker().wake_by_ref();
                                return Poll::Pending;
                            }
                            // update the incoming items
                            if this.update() {
                                log::debug!("new blocker");

                                continue 'ex;
                            }
                        }
                        Poll::Ready(None) => {
                            log::trace!("message receiver empty");
                            // instinctively it suggests that the message
                            // channel is closed
                            //return Poll::
                        }
                    }
                    if this.is_blocked() {
                        log::debug!("New blocker");
                        continue 'ex;
                    }

                    // drive all the actor future into completion
                    //log::info!("runner: {:?} ", this);
                    let mut index = 0;
                    while index < this.inner.len() {
                        let pair = &mut this.inner[index];
                        match pair.inner.as_mut().poll(&mut this.act, &mut this.ctx, cx) {
                            // future is ready
                            Poll::Ready(()) => {
                                // remove completed futures
                                this.inner.swap_remove(index);

                                // the actor state is changed
                                // re-poll it
                                if this.ctx.state != ActorState::Running {
                                    log::debug!("Actor state changed");
                                    cx.waker().wake_by_ref();
                                    return Poll::Pending;
                                }
                                // to see if new blocker arrive or not
                                // if does go the beginning of the loop
                                if this.update() {
                                    continue 'ex;
                                }
                            }
                            // future is not ready
                            Poll::Pending => {
                                // the actor state is changed
                                // re-poll it
                                if this.ctx.state != ActorState::Running {
                                    log::debug!("Actor state changed");
                                    cx.waker().wake_by_ref();
                                    return Poll::Pending;
                                }
                                // to see if new blocker arrive or not
                                // abort some futures if some
                                // if arrive, push the new blocker to
                                // the queue
                                // and go the beginning of the loop
                                if this.update() {
                                    if this.is_blocked() && !this.to_stop() {
                                        // NOTE swap the currrent item with
                                        // the last item to exclude the
                                        // case that polling same item
                                        // all th time
                                        // - it boosts the reliability
                                        //   of the runtime routine
                                        let last = this.inner.len() - 1;
                                        if last != index {
                                            this.inner.swap(index, last);
                                        }
                                        continue 'ex;
                                    }
                                }
                                // if not continue, go to the next item
                                index += 1;
                            }
                        }
                    }

                    // update the actor state
                    /*
                     *log::debug!("{:?}", &this.ctx.pack);
                     *log::debug!(
                     *    "{:?}, {:?}, {:?}, {:?}, {:?}",
                     *    this.ctx.pack.is_alive(),
                     *    this.ctx.pack.is_closing(),
                     *    !this.ctx.state.is_started(),
                     *    !this.inner.is_empty(),
                     *    !this.bloc.is_empty()
                     *);
                     */
                    if !this.is_alive() {
                        log::debug!("Actor closing");
                        this.ctx.state = ActorState::Stopped;
                    }
                    return Poll::Pending;
                }
            }

            // stopping the actor
            // and reject all incoming items
            // from context
            // it can restored into running
            // or turn into stopped status
            ActorState::Stopping => {
                match A::state(&mut this.act, &mut this.ctx) {
                    ActingState::Abort => {
                        // abort the actor
                        this.ctx.state = ActorState::Aborted;
                        log::debug!("Actor Aborted when Stopping");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    ActingState::Resume => {
                        // resume the actor
                        this.ctx.state = ActorState::Running;
                        log::debug!("Actor Resumed when Stopping");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                    _ => {
                        // stop the actor
                        this.ctx.state = ActorState::Stopped;
                        log::debug!("Actor Stopped when Stopping");
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
            }

            // actor is stopped, exit the program in a later
            ActorState::Stopped => {
                // update ContextRunner is not necessary
                // since it is stopped, it is no need to
                // update the new incoming blocker and futures
                // TODO besides that the actor should reject
                // all incoming messages/spawned futures
                A::stopped(&mut this.act, &mut this.ctx);
                // the actor state is changed
                // re-poll it
                if this.ctx.state != ActorState::Stopped {
                    log::warn!("Actor state changed 9i09");
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                log::debug!("Actor successfully Stopped");
                // update ContextRunner is not necessary
                A::close(&mut this.act, &mut this.ctx);
                log::debug!("Actor successfully Closed");
                Poll::Ready(())
            }

            // actor is aborted
            // it will stop the actor immediately
            ActorState::Aborted => {
                // update ContextRunner is not necessary
                // TODO clean-up/refresh the actor and Context
                // and ContextRunner beforehand
                A::aborted(&mut this.act, &mut this.ctx);
                // the actor state is changed
                // re-poll it
                if this.ctx.state != ActorState::Aborted {
                    log::debug!("Actor state changed");
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                log::debug!("Actor Aborted");
                // update ContextRunner is
                // not necessary here
                //
                // even if the actor state is changed
                // after executing it
                // it is ignored though anyway
                // the actor will respond to state change
                // before `Actor::close`
                A::close(&mut this.act, &mut this.ctx);
                log::debug!("Actor successfully Closed");
                Poll::Ready(())
            }
        }
    }
}
