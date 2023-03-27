#![allow(unused)]

use crossbus::{actor::Actor, context::Context, message::Message};

struct Sum {
    sum: i32,
}

#[derive(Debug)]
struct Num(i32);
impl Message for Num {}

impl Actor for Sum {
    type Message = Num;

    fn create(_: &mut Context<Self>) -> Self {
        Self { sum: 0 }
    }

    fn action(&mut self, msg: Self::Message, _: &mut Context<Self>) {
        self.sum += msg.0;
        log::debug!("current sum: {}", self.sum);
    }
}

#[test]
#[cfg(feature = "tokio")]
fn test_routine() {
    use crossbus::{
        actor::Actor,
        context::{Context, ContextRunner},
        reactor::{Reactor, ReactorPair},
        register::{ActorRegister, Register},
        rt::{runtime_tokio::Runtime, Spawning},
    };

    simple_logger::init_with_level(log::Level::Debug).unwrap();
    Runtime::block_on(async {
        log::debug!("use tokio runtime");
        let mut ctx = Context::new();
        let act: Sum = Actor::create(&mut ctx);
        let reg = ActorRegister::new(act);
        let actor_guard = Register::push(reg);
        let addr = ctx.address();
        // send message in another scope
        {
            let sender = addr.sender();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            assert_eq!(sender.message_number(), 3);
        }
        let runner = ContextRunner::new(ctx, actor_guard);
        Reactor::push(ReactorPair::new(runner));
        Reactor::as_future().await;
    });
}

#[test]
#[cfg(feature = "async-std")]
fn test_routine() {
    use crossbus::{
        actor::Actor,
        context::{Context, ContextRunner},
        reactor::{Reactor, ReactorPair},
        register::{ActorRegister, Register},
        //log,
        rt::{runtime_async_std::Runtime, Spawning},
    };

    simple_logger::init_with_level(log::Level::Debug).unwrap();
    Runtime::block_on(async {
        log::debug!("use async std runtime");
        let mut ctx = Context::new();
        let act: Sum = Actor::create(&mut ctx);
        let reg = ActorRegister::new(act);
        let actor_guard = Register::push(reg);
        let addr = ctx.address();
        // send message in another scope
        {
            let sender = addr.sender();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            assert_eq!(sender.message_number(), 3);
        }
        let runner = ContextRunner::new(ctx, actor_guard);
        Reactor::push(ReactorPair::new(runner));
        Reactor::as_future().await;
    });
}

#[test]
#[cfg(feature = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn test_routine() {
    use crossbus::{
        actor::Actor,
        context::{Context, ContextRunner},
        reactor::{Reactor, ReactorPair},
        register::{ActorRegister, Register},
        rt::{runtime_wasm32::Runtime, Spawning},
    };

    wasm_logger::init(wasm_logger::Config::default());
    let _ = Runtime::spawn_local(async {
        log::debug!("use wasm runtime");
        let mut ctx = Context::new();
        let act: Sum = Actor::create(&mut ctx);
        let reg = ActorRegister::new(act);
        let actor_guard = Register::push(reg);
        let addr = ctx.address();
        // send message in another scope
        {
            let sender = addr.sender();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            sender.send(Num(1)).unwrap();
            assert_eq!(sender.message_number(), 3);
            dbg!(sender);
        }
        let runner = ContextRunner::new(ctx, actor_guard);
        Reactor::push(ReactorPair::new(runner));
        Reactor::as_future().await;
    });
}
