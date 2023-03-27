#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! CrossBus is a platform-less runtime-less actor computing model
//!
//! ## Overview
//! [CrossBus](https://github.com/hominee/crossbus) is an implementation of
//! [Actor Computing Model](https://en.wikipedia.org/wiki/Actor_model),
//! with the concept that
//!
//! - **Runtime-less**
//!
//!   crossbus neither provide runtime for app execution
//!   nor access the system interface abstraction.
//!   no built-in runtime, but any runtime is allowed,  
//!   the system-provided / third-party's / that from
//!   `FFI`-binding all work.
//!   Last but not least, even a bare-bone [noop-waker executor](https://docs.rs/futures-task/latest/futures_task/fn.noop_waker.html)
//!   can do.
//!
//!   runtime-located features like `concurrency` and `I/O`
//!   are up to implementor.
//!
//! - **Bare-Metal Compatible**
//!
//!   crossbus links to no std library, no system libraries, no libc,
//!   and a few upstream libraries
//!   enbale you run rust code on bare metal.
//!
//! - **Platform-less by Runtime-less**
//!
//!   take the advantage of runtime-less, crossbus is able to
//!   bypass the limitation of runtime implementor and system
//!   interface abstraction and go right straight to manipulate
//!   task directly.
//!   This is the primary concept of crossbus to run across
//!   many platforms or without platforms.
//!
//! - **Future-oriented Routine and Events**
//!
//!   the futures way to execute task is retained even
//!   without runtime thanks to rust. crossbus defines a set of types
//!   and traits to allow asynchronous tasks manipulation.
//!
//! - **Real-time Execution Control**
//!
//!   taking the advantage of the design of future routine,
//!   each poll and events alongside can be intercepted for
//!   each spawned task during execution, with which more
//!   execution control is possible.
//!

#[macro_use]
extern crate alloc;

pub mod actor;
pub mod address;
pub mod blocker;
pub mod context;
pub mod delayer;
#[cfg(not(feature = "log"))]
#[cfg_attr(docsrs, doc(cfg(not(feature = "log"))))]
#[macro_use]
pub mod log;
pub mod message;
pub(crate) mod queue;
pub mod reactor;
pub mod register;
#[cfg(feature = "rt")]
#[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
pub mod rt;
pub mod stream;
#[cfg(feature = "time")]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
pub mod time;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use crossbus_derive::{main, Message};

pub mod prelude {
    //! Some common traits and types

    pub use crate::actor::{ActingState, Actor, ActorId, ActorState, Future};
    pub use crate::address::{Addr, QueueError, Receiver, Sender};
    pub use crate::context::Context;
    pub use crate::reactor::{ReactingOrder, Reactor, ReactorPair};
    pub use crate::register::{ActorRegister, Register};
    #[cfg(feature = "rt")]
    pub use crate::rt::{self, SpawnJoinHandle, Spawning};
    pub use crate::{actor, address, context, message, reactor, register};
    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    pub use crossbus_derive::{main, Message};
    //pub use crate::blocker::{self, Blocker, BlockerState, Blocking, BlockingState};
    //pub use crate::delayer::{self, Delayer, DelayerState, Delaying, DelayingState};
    //pub use crate::message::{self, MStream, MStreamState, MStreaming, MStreamingState, Message};
    //pub use crate::stream::{self, Stream, StreamState, Streaming, StreamingState};
}
