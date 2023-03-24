//! types for actors Messages Communication and Handling
//!
//! In actor model, actor receives message from other
//! actors, and send messages to other actor as well.
//!
//! From the perspective of an individual actor's message queue,
//! there are many producers and only the host can consume
//! the messages.
//!
//! Roughly speaking,
//! - [Sender](Sender) is the message producer, through what other
//!   actors can send message to self
//!   the message queue closed when  all sender get dropped and reopen
//!   when new sender is created
//!
//! - [Receiver](Receiver) is the message consumer, used inside
//!   of actor itself. The message can be retrived via [Stream](https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html)
//!
//! - [WeakSender](WeakSender) and [WeakReceiver](WeakReceiver)
//!   can be safe reference of its upgrade
//!
//! ```no_run rust
//! struct Num(uisze);
//! impl Message for Num {}
//!
//! struct CrossBus{
//!     sum: isize
//! }
//! impl Actor for CrossBus {
//!     type Message = Num;
//!
//!     fn create(ctx: &mut Context<Self>) -> Self {
//!         Self { sum: 0, }
//!     }
//!
//!     fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
//!         self.sum += msg.0;
//!     }
//!
//! }
//!
//! let (addr1, _) = CrossBus::start();
//! let (addr2, _) = CrossBus::start();
//! let sender1 = addr1.sender();
//! sender1.send(Num(1)).unwrap();
//! sender1.send(Num(2)).unwrap();
//! sender1.send(Num(3)).unwrap();
//! assert_eq!(sender1.message_number(), 3)
//! let sender2 = addr2.sender();
//! sender2.send(Num(-1)).unwrap();
//! sender2.send(Num(-2)).unwrap();
//! sender2.send(Num(-3)).unwrap();
//!
//! ```

extern crate alloc;
#[cfg(not(feature = "log"))]
use crate::log;
use crate::{message::Message, queue::Queue, reactor::inc_poll_budget};
use alloc::sync::{Arc, Weak};
use core::{
    fmt, hint,
    pin::Pin,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
    task::{Context as CoreContext, Poll},
};

use futures_core::{stream::Stream as CoreStream, task::__internal::AtomicWaker};

/// An asynchronous thread-safe mpsc(multiple producer single consumer)
/// channel that used to communicate with between actors and threads
pub fn channel<M: Message>() -> (Sender<M>, Receiver<M>) {
    let pack = Arc::new(Pack {
        messages: Queue::new(),
        num_senders: AtomicUsize::new(0),
        state: PackState::new(),
        capacity: AtomicUsize::new(usize::MAX),
        waker: AtomicWaker::new(),
    });
    let tx = Sender { pack: pack.clone() };
    let rx = Receiver { pack };
    (tx, rx)
}

/// A message producer that deliver messages
/// to the [`Receiver`](`Receiver`)
pub struct Sender<M: Message> {
    pub(crate) pack: Arc<Pack<M>>,
}

impl<M: Message> fmt::Debug for Sender<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").field("pack", &self.pack).finish()
    }
}

/// Weak-referenced Sender
///
/// it does not count on the underlying
/// data ownership, so often used as  
/// a temporary reference to potentially
/// dropped data
pub struct WeakSender<M: Message> {
    pack: Weak<Pack<M>>,
}

impl<M: Message> WeakSender<M> {
    pub fn upgrade(&self) -> Option<Sender<M>> {
        Weak::upgrade(&self.pack).map(|pack| Sender { pack: pack })
    }
}

impl<M: Message> Sender<M> {
    /// create an instance of Sender
    /// from Receiver
    pub fn as_receiver(&self) -> Receiver<M> {
        self.pack.num_senders.fetch_sub(1, Ordering::Relaxed);
        Receiver {
            pack: self.pack.clone(),
        }
    }

    /// get the capcity of the channel
    ///
    /// it will remain the `usize::MAX`
    /// when created, but can be updated
    /// via [`set_capacity`](`Sender::set_capacity`)
    pub fn capacity(&self) -> usize {
        self.pack.capacity()
    }

    /// set the capcity of message queue
    ///
    /// when the messages reach the capcity
    /// it will transimit the queue to
    /// state `STATUSFULL` where no more
    /// messages will be received
    pub fn set_capacity(&self, cap: usize) {
        self.pack.set_capacity(cap)
    }

    /// get the number of messages remain
    /// in the queue
    ///
    /// ```no_run rust
    /// struct Num(uisze);
    /// impl Message for Num {}
    ///
    /// struct CrossBus;
    /// impl Actor for CrossBus {
    ///     ...
    /// }
    ///
    /// let (addr, _) = CrossBus::start();
    /// let sender = addr.sender();
    /// sender.send(Num(1)).unwrap();
    /// sender.send(Num(2)).unwrap();
    /// sender.send(Num(3)).unwrap();
    /// assert_eq!(sender.message_number(), 3)
    /// ```
    pub fn message_number(&self) -> usize {
        self.pack.message_number()
    }

    /// directly push a message into the queue
    ///
    /// the message will be inserted the queue
    /// and notify the receiver to wake
    ///
    /// ```no_run rust
    /// let sender = addr.sender();
    /// sender.send(Num(1));
    /// ```
    pub fn send(&self, msg: M) -> Result<(), QueueError<M>>
    where
        M: Send,
    {
        // to check if the queue is full
        //
        // NOTE that if the `Pack` is in closing pharse,
        // restore the state from PackState::Closing(_)
        // to PackState::Alive(_)
        log::trace!("{:?}", self.pack.state);
        if self.pack.is_full() {
            return Err(QueueError::Full(msg));
        } else if self.pack.is_closed() {
            return Err(QueueError::Closed(msg));
        } else if self.pack.is_closing() {
            self.pack.restore_state();
        }
        self.wake_push(msg);
        Ok(())
    }

    /// downgrade to `WeakSender` which can later be upgraded
    ///
    /// **NOTE** that the underlying data could be
    /// moved or consumed or otherwise
    pub fn downgrade(&self) -> WeakSender<M> {
        WeakSender {
            pack: Arc::downgrade(&self.pack),
        }
    }

    /// Push message to the queue and signal to the receiver
    fn wake_push(&self, msg: M)
    where
        M: Send,
    {
        // Push the message onto the message queue
        self.pack.messages.push(msg);
        self.pack.inc(1);
        self.pack.update_state();
        inc_poll_budget(2);

        // Signal to the receiver that a message has been enqueued.
        self.pack.waker.wake();
    }
}

impl<M: Message> Unpin for Sender<M> {}

impl<M: Message> Clone for Sender<M> {
    fn clone(&self) -> Self {
        let num = self.pack.num_senders.load(Ordering::Acquire);
        loop {
            if num >= self.pack.capacity() - self.pack.message_number() {
                unreachable!("too many senders");
            }
            let next = num + 1;
            let real = self
                .pack
                .num_senders
                .compare_exchange(num, next, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap();
            if real == num {
                return Sender {
                    pack: self.pack.clone(),
                };
            }
        }
    }
}

impl<M: Message> Drop for Sender<M> {
    fn drop(&mut self) {
        let num = self.pack.num_senders.fetch_sub(1, Ordering::Relaxed);
        inc_poll_budget(2);
        log::trace!("sender dropped");
        if num == 1 {
            log::trace!("sender destroyed");
            self.pack.waker.wake();
        }
    }
}

/// A message consumer that poll messages
/// from the [`Sender`](`Sender`)
pub struct Receiver<M: Message> {
    pub(crate) pack: Arc<Pack<M>>,
}

impl<M: Message> fmt::Debug for Receiver<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver")
            .field("pack", &self.pack)
            .finish()
    }
}

impl<M: Message> Receiver<M> {
    /// create an instance of Sender
    /// from Receiver
    pub fn as_sender(&self) -> Sender<M> {
        self.pack.num_senders.fetch_add(1, Ordering::Relaxed);
        self.pack.state.set_alive();
        inc_poll_budget(2);
        Sender {
            pack: self.pack.clone(),
        }
    }

    /// get the capcity of the channel
    ///
    /// it will remain the `usize::MAX`
    /// when created, but can be updated
    /// via [`set_capacity`](`Self::set_capacity`)
    pub fn capacity(&self) -> usize {
        self.pack.capacity()
    }

    /// set the capcity of message queue
    ///
    /// when the messages reach the capcity
    /// it will transimit the queue to
    /// state `STATUSFULL` where no more
    /// messages will be received
    pub fn set_capacity(&self, cap: usize) {
        self.pack.set_capacity(cap)
    }

    /// get the number of messages remain
    /// in the queue
    ///
    /// ```no_run rust
    /// struct Num(uisze);
    /// impl Message for Num {}
    ///
    /// struct CrossBus;
    /// impl Actor for CrossBus {
    ///     ...
    /// }
    ///
    /// let (addr, _) = CrossBus::start();
    /// let sender = addr.sender();
    /// sender.send(Num(1)).unwrap();
    /// sender.send(Num(2)).unwrap();
    /// sender.send(Num(3)).unwrap();
    /// let receiver = addr.receiver();
    /// assert_eq!(receiver.message_number(), 3)
    /// ```
    pub fn message_number(&self) -> usize {
        self.pack.message_number()
    }

    /// mark the underlying data exchange
    /// gets closed.
    ///
    /// **NOTE** that the underlying message queue
    /// wont get closed if any of the following meets:
    /// - at least one sender is alive
    /// - no sender, but messages remains
    pub fn set_close(&self) {
        self.pack.set_close();
    }

    /// downgrade to `WeakReceiver` which can later be upgraded
    ///
    /// **NOTE** that the underlying data could be
    /// moved or consumed or otherwise
    pub fn downgrade(&self) -> WeakReceiver<M> {
        WeakReceiver {
            pack: Arc::downgrade(&self.pack),
        }
    }

    pub(crate) fn next(&mut self) -> Poll<Option<M>> {
        if self.pack.is_closed() {
            return Poll::Ready(None);
        }
        match unsafe { self.pack.messages.pop() } {
            Some(msg) => {
                self.pack.dec(1);
                self.pack.update_state();
                Poll::Ready(Some(msg))
            }
            None => {
                if self.pack.is_closed() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

impl<M: Message> CoreStream for Receiver<M> {
    type Item = M;

    fn poll_next(self: Pin<&mut Self>, cx: &mut CoreContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.next() {
            Poll::Ready(msg) => Poll::Ready(msg),
            Poll::Pending => {
                this.pack.waker.register(cx.waker());
                this.next()
            }
        }
    }
}

impl<M: Message> Drop for Receiver<M> {
    fn drop(&mut self) {
        // close
        self.set_close();
        log::trace!("receiver dropped");

        // Drain the channel of all pending messages
        loop {
            match self.next() {
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => {
                    log::debug!("receiver destroyed");
                    break;
                }
                Poll::Pending => {
                    // If the channel is closed, then there is no need to park.
                    if self.pack.is_closed() {
                        log::debug!("receiver destroyed");
                        break;
                    }

                    // spin loop here to wait for completion
                    hint::spin_loop();
                }
            }
        }
    }
}

/// Weak-referenced Receiver
///
/// it does not count on the underlying
/// data ownership, so often used as  
/// a temporary reference to potentially
/// dropped data
pub struct WeakReceiver<M: Message> {
    pack: Weak<Pack<M>>,
}

impl<M: Message> WeakReceiver<M> {
    /// upgrade the `WeakReceiver` to `Receiver`
    ///
    /// **NOTE** that the underlying data could be
    /// moved or consumed or otherwise
    pub fn upgrade(&self) -> Option<Receiver<M>> {
        Weak::upgrade(&self.pack).map(|pack| Receiver { pack })
    }
}

/// the address of an actor
///
/// returned when actor starts
///
/// it can be transimited into
/// [`Sender`](Sender) and [`Receiver`](Receiver)
pub struct Addr<M: Message> {
    pack: Arc<Pack<M>>,
}

impl<M: Message> Addr<M> {
    /// create an instance of Addr
    /// from the inner pack data
    pub(crate) fn new(pack: Arc<Pack<M>>) -> Self {
        Self { pack }
    }

    /// get the sender from actor address
    ///
    /// ```no_run rust
    /// struct Num(uisze);
    /// impl Message for Num {}
    ///
    /// struct CrossBus;
    /// impl Actor for CrossBus {
    ///     ...
    /// }
    ///
    /// let (addr, _) = CrossBus::start();
    /// let sender = addr.sender();
    /// sender.send(Num(1)).unwrap();
    /// sender.send(Num(2)).unwrap();
    /// sender.send(Num(3)).unwrap();
    /// assert_eq!(sender.message_number(), 3)
    ///
    /// ```
    pub fn sender(&self) -> Sender<M> {
        self.pack.num_senders.fetch_add(1, Ordering::Relaxed);
        self.pack.state.set_alive();
        inc_poll_budget(2);
        Sender {
            pack: self.pack.clone(),
        }
    }

    /// get the receiver from actor address
    ///
    /// ```no_run rust
    /// struct Num(uisze);
    /// impl Message for Num {}
    ///
    /// struct CrossBus;
    /// impl Actor for CrossBus {
    ///     ...
    /// }
    ///
    /// let (addr, _) = CrossBus::start();
    /// let sender = addr.sender();
    /// sender.send(Num(1)).unwrap();
    /// sender.send(Num(2)).unwrap();
    /// sender.send(Num(3)).unwrap();
    /// let receiver = addr.receiver();
    /// assert_eq!(receiver.message_number(), 3)
    /// ```
    pub fn receiver(&self) -> Receiver<M> {
        Receiver {
            pack: self.pack.clone(),
        }
    }
}

impl<M: Message> Into<Sender<M>> for Addr<M> {
    fn into(self) -> Sender<M> {
        self.pack.num_senders.fetch_add(1, Ordering::Relaxed);
        self.pack.state.set_alive();
        inc_poll_budget(2);
        Sender { pack: self.pack }
    }
}

impl<M: Message> Into<Receiver<M>> for Addr<M> {
    fn into(self) -> Receiver<M> {
        Receiver { pack: self.pack }
    }
}

const STATUSCLOSED: u8 = 0b0000_0001;
const STATUSFULL: u8 = 0b0000_0010;
const STATUSCLOSING: u8 = 0b0000_0100;
const STATUSALIVE: u8 = 0b0000_1000;

#[derive(Debug)]
pub(crate) struct PackState {
    status: AtomicU8,
    num: AtomicUsize,
}

impl PackState {
    pub(super) fn new() -> Self {
        PackState {
            status: AtomicU8::new(STATUSALIVE),
            num: AtomicUsize::new(0),
        }
    }

    pub(super) fn is_closed(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUSCLOSED
    }

    pub(super) fn set_close(&self) {
        log::info!("close the pack",);
        self.status.store(STATUSCLOSED, Ordering::Relaxed);
    }

    pub(super) fn is_closing(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUSCLOSING
    }

    pub(super) fn set_closing(&self) {
        self.status.store(STATUSCLOSING, Ordering::Relaxed);
    }

    pub(super) fn is_full(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUSFULL
    }

    pub(super) fn set_full(&self) {
        self.status.store(STATUSFULL, Ordering::Relaxed);
    }

    pub(super) fn is_alive(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUSALIVE
    }

    pub(super) fn set_alive(&self) {
        self.status.store(STATUSALIVE, Ordering::Relaxed);
    }

    pub(super) fn message_number(&self) -> usize {
        self.num.load(Ordering::Relaxed)
    }

    pub(super) fn inc(&self, delta: usize) {
        if self.is_full() || self.is_closed() {
            unreachable!("push message to Closed/Full Queue")
        }
        self.num.fetch_add(delta, Ordering::Relaxed);
    }

    pub(super) fn dec(&self, delta: usize) {
        if self.is_closed() {
            unreachable!("pop message to Closed Queue")
        } else if self.message_number() == 0 {
            unreachable!("pop message from empty Queue")
        }
        self.num.fetch_sub(delta, Ordering::Relaxed);
    }
}

impl Clone for PackState {
    fn clone(&self) -> Self {
        Self {
            status: AtomicU8::new(self.status.load(Ordering::Relaxed)),
            num: AtomicUsize::new(self.num.load(Ordering::Relaxed)),
        }
    }
}

impl PartialEq for PackState {
    fn eq(&self, other: &Self) -> bool {
        self.status.load(Ordering::Relaxed) == other.status.load(Ordering::Relaxed)
            && self.num.load(Ordering::Relaxed) == other.num.load(Ordering::Relaxed)
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

pub(crate) struct Pack<M>
where
    M: Message,
{
    messages: Queue<M>,
    num_senders: AtomicUsize,
    state: PackState,
    capacity: AtomicUsize,
    // wakers to the receiver's task.
    waker: AtomicWaker,
}

impl<M: Message> Pack<M> {
    /// Create an instance of Pack and
    /// Guard it with Arc
    pub fn new() -> Arc<Pack<M>> {
        Arc::new(Pack {
            messages: Queue::new(),
            num_senders: AtomicUsize::new(0),
            state: PackState::new(),
            capacity: AtomicUsize::new(usize::MAX),
            waker: AtomicWaker::new(),
        })
    }

    pub fn is_closed(&self) -> bool {
        self.state.is_closed()
    }

    pub fn set_close(&self) {
        self.state.set_close()
    }

    pub fn is_closing(&self) -> bool {
        self.state.is_closing()
    }

    pub fn set_closing(&self) {
        self.state.set_closing()
    }

    pub fn is_full(&self) -> bool {
        self.state.is_full()
    }

    pub fn set_full(&self) {
        self.state.set_full()
    }

    pub fn is_alive(&self) -> bool {
        self.state.is_alive()
    }

    pub fn set_alive(&self) {
        self.state.set_alive()
    }

    pub fn message_number(&self) -> usize {
        self.state.message_number()
    }

    pub fn sender_number(&self) -> usize {
        self.num_senders.load(Ordering::Relaxed)
    }

    pub(crate) fn inc_sender(&self, delta: usize) {
        self.num_senders.fetch_add(delta, Ordering::Relaxed);
    }

    pub(super) fn inc(&self, delta: usize) {
        self.state.inc(delta)
    }

    pub fn dec(&self, delta: usize) {
        self.state.dec(delta)
    }

    /// update the `Pack` state
    /// - `Closed` when all message is consumed
    /// - `Alive` when created/ at default
    /// - `Closing` when all senders are closed OR
    ///    PackState::set_close is called
    /// - `Full` when the messages reach the capcity
    pub fn update_state(&self) {
        if self.is_closed() {
            return;
        }
        let num = self.state.message_number();
        let num_senders = self.sender_number();
        if num_senders == 0 {
            if num != 0 {
                log::debug!("no sender, but message remains, closing");
                self.set_closing();
            } else {
                log::debug!("no sender closed");
                self.set_close();
            }
            return;
        }
        //if self.state.is_closing() {
        //if num == 0 {
        //println!("no message and is closing, closed");
        //self.state.set_close();
        //}
        //} else
        if self.state.is_alive() {
            let capacity = self.capacity.load(Ordering::Relaxed);
            if num > capacity {
                log::debug!("full message");
                self.set_full();
            }
        }
    }

    /// sender sends message to the queue
    /// when pack is in PackState::Closing(_)
    ///
    /// restore the state back to PackState::Alive(_)
    pub fn restore_state(&self) -> bool {
        if self.state.is_closing() {
            self.state.set_alive();
            return true;
        }
        log::warn!(
            "call Pack::restore_state in {:?} (`PackState::Closing(_)` ONLY)",
            self.state
        );
        false
    }

    pub fn capacity(&self) -> usize {
        self.capacity.load(Ordering::Relaxed)
    }

    pub fn set_capacity(&self, cap: usize) {
        self.capacity.store(cap, Ordering::Relaxed)
    }

    pub fn next(&self) -> Poll<Option<M>> {
        if self.is_closed() {
            return Poll::Ready(None);
        }
        match unsafe { self.messages.pop() } {
            Some(msg) => {
                self.dec(1);
                self.update_state();
                Poll::Ready(Some(msg))
            }
            None => {
                if self.is_closed() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }

    pub fn poll_next(&self, cx: &mut CoreContext<'_>) -> Poll<Option<M>> {
        match self.next() {
            Poll::Ready(msg) => Poll::Ready(msg),
            Poll::Pending => {
                self.waker.register(cx.waker());
                self.next()
            }
        }
    }
}

impl<M: Message> fmt::Debug for Pack<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pack")
            .field("messages", &self.message_number())
            .field("num_senders", &self.sender_number())
            .field("state", &self.state)
            .field("capacity", &self.capacity())
            .finish()
    }
}

/// error types that occurs when sending
/// an message to the queue.
pub enum QueueError<T> {
    /// the queue reach the capcity
    /// and gets full
    ///
    /// it will reject the message
    /// and return it
    Full(T),
    /// the queue is closed and
    /// no longer receives message
    ///
    /// it will reject the message
    /// and return it
    Closed(T),
}

impl<T> fmt::Debug for QueueError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => f.write_str("Queue Closed"),
            Self::Full(_) => f.write_str("Queue Full"),
        }
    }
}
