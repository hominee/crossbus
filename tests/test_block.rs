#![allow(unused)]
#![cfg(all(feature = "derive", feature = "force-poll"))]

mod common;
use core::sync::atomic::{AtomicBool, Ordering};
// each test module only run it ONCE
// used the static bool to assure
// the logger initialized only once
// when the same feature enabled
static LOGGER_INIT: AtomicBool = AtomicBool::new(false);

use crossbus::{
    actor::{Actor, ActorState, Handle, Localizer},
    blocker::Blocking,
    context::Context,
    reactor::Reactor,
    register::{ActorRegister, Register},
    stream::{Stream, Streaming},
    Message,
};

use core::{
    pin::Pin,
    sync::atomic::AtomicUsize,
    task::{Context as CoreContext, Poll},
    time::Duration,
};
use futures_core::stream;

#[derive(Debug)]
struct Summer {
    sum: i32,
    handle: Option<Handle>,
    swith_block: bool,
    index: usize,
    start_at: f64,
}

#[derive(Debug)]
struct Num(i32);

#[derive(Debug, Message)]
enum Order {
    Number(Num),
    Bloc(Bloc),
}

#[derive(Debug)]
struct Bloc(f32);

impl Actor for Summer {
    type Message = Order;

    fn create(ctx: &mut Context<Self>) -> Self {
        Self {
            sum: 0,
            handle: None,
            swith_block: false,
            index: 0,
            start_at: 0.0,
        }
    }

    fn started(&mut self, ctx: &mut Context<Self>) {
        let sender = ctx.sender();
        let mut items = vec![
            Order::Number(Num(1)),
            Order::Number(Num(1)),
            Order::Number(Num(1)),
            Order::Bloc(Bloc(1.3)),
            Order::Number(Num(1)),
            Order::Number(Num(1)),
            Order::Number(Num(1)),
            Order::Bloc(Bloc(0.7)),
            Order::Number(Num(1)),
            Order::Number(Num(1)),
            Order::Number(Num(1)),
        ];
        while let Some(item) = items.pop() {
            sender.send(item).unwrap();
        }
    }

    fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
        match msg {
            Order::Number(num) => {
                self.sum += num.0;
                self.index += 1;
            }
            Order::Bloc(bloc) => {
                log::debug!("got bloc with duration: {}", bloc.0);
                let now = common::get_now();
                self.start_at = now;
                let amt = bloc.0 as f64;
                if !self.swith_block {
                    let dur = Duration::from_secs_f64(amt);
                    #[cfg(feature = "tokio")]
                    let sleep_fut = tokio::time::sleep(dur);
                    #[cfg(feature = "async-std")]
                    let sleep_fut = async_std::task::sleep(dur);
                    #[cfg(feature = "wasm32")]
                    let sleep_fut = crossbus::rt::wasm_timeout::sleep(dur);
                    let local = Localizer::new(sleep_fut);
                    log::info!("normal future Block at {} with index: {}", now, self.index,);
                    self.swith_block = !self.swith_block;
                    ctx.blocking(local);
                } else {
                    // blocking from blocker duration
                    #[cfg(any(feature = "async-std", feature = "tokio"))]
                    ctx.blocking_duration::<std::time::Instant>(Duration::from_secs_f64(amt));
                    #[cfg(feature = "wasm32")]
                    ctx.blocking_duration::<common::time_wasm32::Clock>(Duration::from_secs_f64(
                        1.3,
                    ));
                    log::info!("duration Block at {} with index: {}", now, self.index,);
                }
                self.index += 1;
            }
        }
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {}
}

impl Blocking<Summer> for Summer {
    fn finished(&mut self, ctx: &mut Context<Self>) {
        let now = common::get_now();
        log::info!(
            "Blocker ends at {} with index: {}, whole elapsed: {}",
            now,
            self.index,
            now - self.start_at
        );
    }
}

#[cfg(feature = "tokio")]
fn replace_tokio_runtime() {
    use crossbus::rt::runtime_tokio::TokioRuntime;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    TokioRuntime::set_runtime(rt);
}

#[test]
#[cfg(feature = "tokio")]
fn test_block() {
    use crossbus::rt::{runtime_tokio::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        //common::init_module_level("test_block", log::Level::Debug);
        common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    // blocking from normal future
    // since crossbus provides bare tokio runtime
    // at default which donot support
    // - time
    // - io
    // - multi-thread
    // So we need to replace it with
    replace_tokio_runtime();

    #[crossbus::main(runtime = tokio)]
    async fn main() {
        let _ = Summer::start();
    }
    main();
}

#[test]
#[cfg(feature = "async-std")]
fn test_block() {
    use crossbus::rt::{runtime_async_std::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        //common::init_module_level("test_block", log::Level::Debug);
        common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    #[crossbus::main(runtime = async-std)]
    async fn main() {
        let _ = Summer::start();
    }
    main();
}

#[cfg(any(feature = "wasm32"))]
#[wasm_bindgen_test::wasm_bindgen_test]
fn test_block() {
    use crossbus::rt::{runtime_wasm32::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        common::init_module_level("test_block", log::Level::Debug);
        //common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
    #[crossbus::main(runtime = wasm32)]
    async fn main() {
        let _ = Summer::start();
    }
    main();
}
