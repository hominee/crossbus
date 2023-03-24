//! An abstraction for a computational entity
//!
use core::{
    any::Any,
    future::Future as CoreFuture,
    marker::PhantomData,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context as CoreContext, Poll},
};
use pin_project_lite::pin_project;

#[cfg(not(feature = "log"))]
use crate::log;
use crate::{
    address::Addr,
    context::Context,
    message::Message,
    register::{ActorRegister, Register},
};

/// Actor State
#[derive(PartialEq, Clone, Debug)]
pub enum ActorState {
    Created,
    Started,
    Running,
    Aborted,
    //Paused,
    //Resumed,
    Stopping,
    Stopped,
}

/// Indicator to direct the Actor
/// at runtime,
/// - `Continue`: just continue the execution, no
///   intercept happens
/// - `Abort`: intercept the signal and
///   exit execution immediately
/// - `Stop`: intercept the execution and
///   step the actor into `Stopping` state
///   if `Resume` is not returned following, then
///   the actor get `Stopped`
/// - `Resume`: restore the Actor back
///   to `Running` from `Stopping` state
#[derive(PartialEq, Clone, Debug)]
pub enum ActingState {
    Continue,
    Abort,
    Resume,
    Stop,
}

impl ActorState {
    pub fn is_running(&self) -> bool {
        *self == ActorState::Running || *self == ActorState::Stopping
    }

    pub fn is_stopping(&self) -> bool {
        *self == ActorState::Stopping || *self == ActorState::Stopped
    }

    pub fn is_started(&self) -> bool {
        *self != Self::Created
    }
}

static HANDLECOUNT: AtomicUsize = AtomicUsize::new(1);

/// An unique identity
///
/// **NOTE** that if the inner id is `0`
/// it means that the returned
/// handle is not valid, and the
/// corresponding future not spawned
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Handle(usize);

impl Handle {
    pub(crate) fn new() -> Self {
        Self(HANDLECOUNT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn inner(self) -> usize {
        self.0
    }

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// An abstraction for Actor's Future routine
///
pub trait Future<A>
where
    A: Actor,
{
    /// the returned type of value when the future polled successfully
    type Output;

    fn poll(
        self: Pin<&mut Self>,
        act: &mut A,
        ctx: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<Self::Output>;

    fn downcast_ref(&self) -> Option<&dyn Any>;

    fn downcast_mut(self: Pin<&mut Self>) -> Option<Pin<&mut dyn Any>>;
}

impl<A: Actor, F, Output> Future<A> for F
where
    F: Unpin + FnMut(&mut A, &mut Context<A>, &mut CoreContext<'_>) -> Poll<Output>,
{
    type Output = Output;

    fn poll(
        mut self: Pin<&mut Self>,
        act: &mut A,
        ctx: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<Self::Output> {
        (self)(act, ctx, cx)
    }

    fn downcast_ref(&self) -> Option<&dyn Any> {
        None
    }

    fn downcast_mut(self: Pin<&mut Self>) -> Option<Pin<&mut dyn Any>> {
        None
    }
}

pin_project! {
    /// A Converter of normal future into
    /// actor Future
    pub struct Localizer<A, F>
    where
        A: Actor,
        F: CoreFuture
    {
        #[pin]
        future: F,
        _data: PhantomData<A>,
    }
}

impl<A, F> Localizer<A, F>
where
    F: CoreFuture,
    A: Actor,
{
    pub fn new(future: F) -> Self {
        Localizer {
            future,
            _data: PhantomData,
        }
    }
}

impl<A, F> Future<A> for Localizer<A, F>
where
    F: CoreFuture,
    A: Actor,
{
    type Output = F::Output;

    fn poll(
        self: Pin<&mut Self>,
        _: &mut A,
        _: &mut Context<A>,
        cx: &mut CoreContext<'_>,
    ) -> Poll<Self::Output> {
        self.project().future.poll(cx)
    }

    fn downcast_ref(&self) -> Option<&dyn Any> {
        None
    }

    fn downcast_mut(self: Pin<&mut Self>) -> Option<Pin<&mut dyn Any>> {
        None
    }
}

/// unique actor id,
///
/// **NOTE** that each actor is assigned
/// an unique id after [`Actor::start`](Actor::start)
///
/// and store it into [Context](Context)
pub type ActorId = usize;

/// An abstraction for a computational entity
///
pub trait Actor: Sized + Unpin + 'static {
    type Message: Message;

    /// create a instance of an actor
    fn create(ctx: &mut Context<Self>) -> Self;

    /// preparation before start the actor
    /// it get executed before `Actor::state`
    /// gets called.  That means it is called even
    /// `Actor::state` return `ActingState::Abort`
    ///
    /// even if the actor state is changed
    /// after executing it
    /// it is ignored though anyway
    /// the actor should step into Started state
    /// for this state change
    fn initial(&mut self, _: &mut Context<Self>) {}

    /// start the actor
    /// it basically does the following things:
    /// - Create the default context
    /// - Create the actor instance
    /// - consume the Actor to Create an ActorRegister
    /// - Register the Actor and return the ActorGuard
    /// - Invoke ContextRunner to run the actor
    fn start() -> (Addr<Self::Message>, ActorId) {
        let mut ctx = Context::new();
        let act = Self::create(&mut ctx);
        let reg = ActorRegister::new(act);
        let id = reg.id();
        ctx.set_id(id);
        let actor = Register::push(reg);
        (ctx.run(actor), id)
    }

    /// called when the actor starts but not running
    fn started(&mut self, _: &mut Context<Self>) {}

    /// make a reaction when the message comes
    /// called when the actor received message
    fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>);

    /// to decide whether to resume/stop/continue the actor
    /// the actor will step into `Stopping` phase if `Stop`
    /// is returned
    /// and actor will be stopped if `Resume` is not returned
    /// when it is in stopping phase
    fn state(&mut self, _ctx: &mut Context<Self>) -> ActingState {
        ActingState::Continue
    }

    /// called when the actor get aborted
    fn aborted(&mut self, _: &mut Context<Self>) {}

    /// called when the actor is stopping but not to exit
    /// the actor can restore running state from stopped state
    fn stopped(&mut self, _: &mut Context<Self>) {}

    /// final work before exit
    /// it will be called even the actor gets aborted
    /// after this, the actor is terminated
    ///
    /// even if the actor state is changed
    /// after executing it
    /// it is ignored though anyway
    /// the actor will respond to state change
    /// before `Actor::close`
    fn close(&mut self, ctx: &mut Context<Self>) {
        let id = ctx.id();
        Register::get(id).map(|en| en.set_closed(true));
        log::debug!("mark the actor register closed");
    }
}
