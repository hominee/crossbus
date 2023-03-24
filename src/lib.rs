//#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
// TODO: Talk about the features in details
//! CrossBus is a platform-less runtime-less actor computing model
//! - Runtime-less
//! - Platform-less
//! - Bare-Metal compatible
//! - Future-oriented routine and events
//! - Real-time Execution Control
//!
//!

#[macro_use]
extern crate alloc;

pub mod actor;
pub mod address;
pub mod blocker;
pub mod context;
pub mod delayer;
//#[cfg(not(feature = "rt"))]
//#[cfg_attr(docsrs, doc(cfg(not(feature = "rt"))))]
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
pub use derive::{main, Message};

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
    pub use derive::{main, Message};
    //pub use crate::blocker::{self, Blocker, BlockerState, Blocking, BlockingState};
    //pub use crate::delayer::{self, Delayer, DelayerState, Delaying, DelayingState};
    //pub use crate::message::{self, MStream, MStreamState, MStreaming, MStreamingState, Message};
    //pub use crate::stream::{self, Stream, StreamState, Streaming, StreamingState};
}
