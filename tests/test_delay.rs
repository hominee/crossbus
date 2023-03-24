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
    context::Context,
    delayer::Delaying,
    reactor::Reactor,
    Message,
};

use core::{
    pin::Pin,
    task::{Context as CoreContext, Poll},
    time::Duration,
};

const DUR_DELAYED: f64 = 0.5;
const DUR_FINITE: f64 = 0.3;
const DUR_INF: f64 = 0.7;

#[derive(Message, Debug)]
struct Msg {
    typ: MsgType,
    content: &'static str,
}

#[derive(Debug)]
enum MsgType {
    Instant,
    Delayed,
    Finite(usize),
    Infinite,
}

struct Narrator {
    instant: Desc,
    delayed: Desc,
    finite: Desc,
    inf: Desc,
}

struct Desc {
    used: usize,
    send_at: Option<f64>,
    receive_at: Option<f64>,
}

impl Desc {
    fn new() -> Self {
        Self {
            used: 0,
            send_at: None,
            receive_at: None,
        }
    }
}

impl Delaying<Narrator> for Narrator {}

impl Actor for Narrator {
    type Message = Msg;

    fn create(ctx: &mut Context<Self>) -> Self {
        Self {
            instant: Desc::new(),
            delayed: Desc::new(),
            finite: Desc::new(),
            inf: Desc::new(),
        }
    }

    fn started(&mut self, ctx: &mut Context<Self>) {
        let sender = ctx.sender();
        let mut items = vec![
            Msg {
                typ: MsgType::Instant,
                content: "instant msg",
            },
            Msg {
                typ: MsgType::Delayed,
                content: "delayed msg",
            },
            Msg {
                typ: MsgType::Finite(3),
                content: "repeat 3 msg",
            },
            Msg {
                typ: MsgType::Infinite,
                content: "loop msg",
            },
        ];
        let now = common::get_now();
        while let Some(item) = items.pop() {
            match item.typ {
                MsgType::Instant => {
                    ctx.instant_message(item);
                    self.instant.send_at = Some(now);
                }
                MsgType::Delayed => {
                    self.delayed.send_at = Some(now);
                    let dur = Duration::from_secs_f64(DUR_DELAYED);
                    #[cfg(any(feature = "async-std", feature = "tokio"))]
                    ctx.delay_message::<std::time::Instant>(item, dur);
                    #[cfg(feature = "wasm32")]
                    ctx.delay_message::<common::time_wasm32::Clock>(item, dur);
                }
                MsgType::Finite(num) => {
                    self.finite.send_at = Some(now);
                    let dur = Duration::from_secs_f64(DUR_FINITE);
                    #[cfg(any(feature = "async-std", feature = "tokio"))]
                    unsafe {
                        ctx.repeat_message::<std::time::Instant>(item, Some(dur), Some(num))
                    };
                    #[cfg(feature = "wasm32")]
                    unsafe {
                        ctx.repeat_message::<common::time_wasm32::Clock>(item, Some(dur), Some(num))
                    };
                }
                MsgType::Infinite => {
                    self.inf.send_at = Some(now);
                    let dur = Duration::from_secs_f64(DUR_INF);
                    #[cfg(any(feature = "async-std", feature = "tokio"))]
                    unsafe {
                        ctx.repeat_message::<std::time::Instant>(item, Some(dur), None)
                    };
                    #[cfg(feature = "wasm32")]
                    unsafe {
                        ctx.repeat_message::<common::time_wasm32::Clock>(item, Some(dur), None)
                    };
                }
            }
        }
    }

    fn action(&mut self, msg: Self::Message, ctx: &mut Context<Self>) {
        let now = common::get_now();
        match msg.typ {
            MsgType::Instant => {
                self.instant.used += 1;
                self.instant.receive_at = Some(now);
                log::info!(
                    "instant message: {:?}, elapsed: {}",
                    msg,
                    self.instant.receive_at.unwrap() - self.instant.send_at.unwrap()
                );
            }
            MsgType::Delayed => {
                self.delayed.used += 1;
                self.delayed.receive_at = Some(now);
                log::info!(
                    "{:?}, elapsed: {}",
                    msg,
                    now - self.delayed.send_at.unwrap()
                );
            }
            MsgType::Finite(num) => {
                self.finite.used += 1;
                self.finite.receive_at = Some(now);
                log::info!(
                    "{:?}, elapsed: {} secs with {} items",
                    msg,
                    now - self.finite.send_at.unwrap(),
                    self.finite.used
                );
            }
            MsgType::Infinite => {
                self.inf.used = self.inf.used + 1;
                self.inf.receive_at = Some(now);
                log::info!(
                    "{:?}, elapsed: {}, Infinite loop, now: {}",
                    msg,
                    now - self.inf.send_at.as_ref().unwrap(),
                    self.inf.used,
                );
            }
        }
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
fn test_delay() {
    use crossbus::rt::{runtime_tokio::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        common::init_module_level("test_delay", log::Level::Debug);
        //common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    // since crossbus provides bare tokio runtime
    // at default which donot support
    // - time
    // - io
    // - multi-thread
    // So we need to replace it with
    //#[cfg(feature = "tokio")]
    //replace_tokio_runtime();

    #[crossbus::main(runtime = tokio)]
    async fn main() {
        let _ = Narrator::start();
    }
    main();
}

#[test]
#[cfg(feature = "async-std")]
fn test_delay() {
    use crossbus::rt::{runtime_async_std::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        common::init_module_level("test_delay", log::Level::Debug);
        //common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }

    // since crossbus provides bare tokio runtime
    // at default which donot support
    // - time
    // - io
    // - multi-thread
    // So we need to replace it with
    //#[cfg(feature = "tokio")]
    //replace_tokio_runtime();

    #[crossbus::main(runtime = async-std)]
    async fn main() {
        let _ = Narrator::start();
    }
    main();
}

#[cfg(any(feature = "wasm32"))]
#[wasm_bindgen_test::wasm_bindgen_test]
fn test_delay() {
    use crossbus::rt::{runtime_wasm32::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        //common::init_module_level("test_delay", log::Level::Debug);
        common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
    #[crossbus::main(runtime = wasm32)]
    async fn main() {
        let _ = Narrator::start();
    }
    main();
}
