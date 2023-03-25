//! loop a message in a 6 actor-linked cycle
//! for 7 rounds
//! the message sending order is:
//! 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 1
//! the message will be dropped once it is done
use core::{
    any::TypeId,
    sync::atomic::{AtomicUsize, Ordering},
};
use crossbus::prelude::*;

const ACTOR_NUM: usize = 6;
const MSG_NUM: usize = 7;

struct Station {
    // id of the actor
    id: ActorId,
    // number of the actor
    // passes the message
    pass: usize,
    // actor to send message to
    send_to: Option<Sender<Msg>>,
    // actor that send messages
    // used to prevent message
    // queue get closed
    current: Option<Sender<Msg>>,
}

#[derive(Debug, Message)]
struct Msg(usize);

impl Actor for Station {
    type Message = Msg;

    fn create(_: &mut Context<Self>) -> Self {
        Self {
            id: 0,
            pass: 0,
            send_to: None,
            current: None,
        }
    }

    fn action(&mut self, mut msg: Self::Message, ctx: &mut Context<Self>) {
        static STARTER: AtomicUsize = AtomicUsize::new(0);

        if self.id == 0 {
            self.id = ctx.id();
        }
        let len = Register::as_ref()
            .iter()
            .filter(|en| en.type_id() == TypeId::of::<Self>())
            .collect::<Vec<_>>()
            .len();
        log::debug!("current number of actors: {}", len);
        // create new actor as the target receiver
        if len < ACTOR_NUM {
            let new_addr = Station::start();
            self.send_to = Some(new_addr.0.sender());
            log::info!("create a new actor {} by actor {}", new_addr.1, self.id);
        }
        if len == 1 && self.pass == 0 {
            STARTER.store(self.id, Ordering::Relaxed);
            log::info!("mark the first actor");
        } else if len == ACTOR_NUM && self.send_to.is_none() && self.pass == 0 {
            let starter_id = STARTER.load(Ordering::Relaxed);
            let act = Register::as_ref()
                .iter()
                .find(|en| en.downcast_ref::<Self>().unwrap().id == starter_id)
                .and_then(|en| en.downcast_ref::<Self>());
            assert!(act.is_some());
            self.send_to = act.unwrap().current.clone();
            log::info!("mark the last actor");
        }
        msg.0 += 1;
        self.pass += 1;
        self.current = Some(ctx.sender());
        if self.pass < MSG_NUM {
            let sender = self.send_to.as_ref().unwrap();
            log::info!(
                "actor: {} {}-th time send msg: {:?} ",
                self.id,
                self.pass,
                msg,
            );
            sender.send(msg).unwrap();
        } else if self.pass == MSG_NUM {
            let sender = self.send_to.take().unwrap();
            if msg.0 != MSG_NUM * ACTOR_NUM {
                log::info!(
                    "last round, actor: {} {}-th time send msg: {:?} ",
                    self.id,
                    self.pass,
                    msg,
                );
                sender.send(msg).unwrap();
            } else {
                log::info!(
                    "last round and last time, actor: {} {}-th time send msg: {:?} ",
                    self.id,
                    self.pass,
                    msg,
                );
                self.send_to = None;
            }
            log::info!("dorpping sender to exit actor {}", self.id);
            self.current = None;
            assert!(self.send_to.is_none());
            assert!(self.current.is_none());
        }
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        log::info!("actor {} stopped, pass: {}, ", self.id, self.pass);
    }
}

// with async std runtime
#[cfg(feature = "async-std")]
#[crossbus::main(runtime = async-std)]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Station::start();
    let sender = addr.sender();
    sender.send(Msg(0)).unwrap();
}

// with tokio runtime
#[cfg(feature = "tokio")]
#[crossbus::main(runtime = tokio)]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Station::start();
    let sender = addr.sender();
    sender.send(Msg(0)).unwrap();
}

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
fn main() {}
