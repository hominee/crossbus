#![allow(unused)]

/// **README** before running the tests
///
/// in this integration testing,
/// we use `St` to simulate a stream
/// that comes with specific gap (see `St::poll_next`)
/// and the stream will timeout if
/// - the next item comes exceeding 2 secs
/// - the whole stream lasts more than 5 secs
///
/// features to be tested for actor stream
/// - stream paused
/// - stream resume from paused
/// - stream item timeout with given standard
/// - whole stream timeout with given standard
///
/// go the `Actor::state` and comment or comment out some
/// block of code to disable/enable features,
/// you can change the timeout args in `Actor::create`
/// and the way the stream sending its item
/// to see the difference
///
mod common;
use core::sync::atomic::{AtomicBool, Ordering};
// each test module only run it ONCE
// used the static bool to assure
// the logger initialized only once
// when the same feature enabled
static LOGGER_INIT: AtomicBool = AtomicBool::new(false);

use crossbus::{
    actor::{Actor, Handle},
    context::Context,
    stream::{Stream, Streaming},
    Message,
};

use core::{
    pin::Pin,
    task::{Context as CoreContext, Poll},
};
use futures_core::stream;

#[derive(Debug)]
struct Sum {
    sum: i32,
    handle: Option<Handle>,

    /// args for
    /// stream item pause/resume timeout
    loc: bool,
    counter: usize,
    pause_dur: f64,
    pause_at: Option<f64>,

    /// args for
    /// whole stream timeout
    last_dur: f64,
    start_at: Option<f64>,
}

#[derive(Debug, Message)]
struct Num(i32);

#[derive(Debug, Message)]
enum Order {
    Number(Num),
    //Pause(Pause),
    //Resume(Resume),
}

struct St {
    items: Vec<Num>,
    /// Vec<(index-to-wait, duration-to-wait)>
    anchor: Vec<(usize, f32)>,
    /// time deadline when to fire stream item
    deadline: Option<f64>,
    index: usize,
}

impl stream::Stream for St {
    type Item = Num;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut CoreContext<'_>) -> Poll<Option<Self::Item>> {
        if self.items.is_empty() {
            return Poll::Ready(None);
        }
        if self.anchor.is_empty() {
            let item = self.items.remove(0);
            self.index += 1;
            assert!(self.deadline.is_none());
            log::debug!("empty anchor, St left: {:?}", self.items);
            return Poll::Ready(Some(item));
        }

        let (ind, dur) = self.anchor[0];
        // right place
        if self.index == ind {
            //check the current time if precedent
            let ins = get_now();
            if self.deadline.is_none() {
                self.deadline = Some(ins + dur as f64);
                log::info!("set deadline: {:?}", self.deadline);
            }
            if self.deadline.unwrap() > ins {
                Poll::Pending
            } else {
                let item = self.items.remove(0);
                self.anchor.remove(0);
                self.index += 1;
                self.deadline = None;
                log::info!("reset deadline");
                log::debug!("timeout done, st left: {:?}", self.items);
                Poll::Ready(Some(item))
            }
        } else {
            let item = self.items.remove(0);
            self.index += 1;
            assert!(self.deadline.is_none());
            //self.deadline = None;
            log::debug!("stream now, st left: {:?}", self.items);
            Poll::Ready(Some(item))
        }
    }
}

impl Actor for Sum {
    type Message = ();

    fn create(_: &mut Context<Self>) -> Self {
        Self {
            sum: 0,
            handle: None,
            counter: 0,
            loc: false,
            // NOTE: change the args below
            // to see different pause duration
            pause_dur: 0.01,
            pause_at: None,
            // NOTE: change the args below
            // to make it timeout with
            // different gap
            last_dur: 4.0,
            start_at: None,
        }
    }

    fn started(&mut self, ctx: &mut Context<Self>) {
        let items = vec![
            Num(1), // index: 0
            Num(2), // index: 1 wait here for 1s
            Num(1), // index: 2
            Num(3), // index: 3 wait here for 1s
            Num(1), // index: 4
            Num(5), // index: 5 pause/resume here
            Num(1), // index: 6
            Num(1), // index: 7
            Num(1), // index: 8
            Num(1), // index: 9 timeout here for 3s
        ];
        let anchor = vec![(1, 1.0), (3, 1.0), (9, 3.0)];
        let st = St {
            items,
            anchor,
            deadline: None,
            index: 0,
        };
        //let handle = self.spawn_stream(ctx, st);
        let handle = ctx.streaming(st);
        if handle.inner() != 0 {
            self.handle = Some(handle);
        }
    }

    fn action(&mut self, _msg: Self::Message, _: &mut Context<Self>) {
        log::error!("message should not be here, and handled by Stream::action");
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        // restart the Actor to spawn a new
    }
}

use crossbus::stream::{StreamState, StreamingState};

impl Stream<Num> for Sum {
    fn started(&mut self, _: &mut Context<Self>) {
        let now = get_now();
        // the stream is started
        self.start_at.replace(now);
    }

    /// right here is the place where
    /// control the pause/resumption/abortion
    /// of the stream
    fn state(&mut self, ctx: &mut Context<Self>) -> StreamingState {
        // ====================================
        // Comment this out to enable the whole
        // stream timeout
        // ====================================
        // check whole stream timeout at first

        /*
         *let now = get_now();
         *assert!(self.start_at.is_some());
         *if self.start_at.unwrap() + self.last_dur < now {
         *    // exceeding the timeout
         *    log::error!("stream TIMEOUT");
         *    return StreamingState::Abort;
         *}
         */

        // ====================================
        // Comment this out to enable/disable
        // the stream item timeout
        // ====================================
        // check stream item timeout
        if self.counter == 5 {
            let st = ctx.downcast_ref::<Streaming<Self, St>>(self.handle.unwrap());
            assert!(st.is_some());
            let st = st.unwrap();
            if st.state() == StreamState::Paused {
                let ins = get_now();
                if self.pause_at.is_none() {
                    // first pause
                    self.pause_at = Some(ins);
                }
                if self.pause_at.unwrap() + self.pause_dur > ins {
                    // as long as the `Resume` state is not
                    // returned, the stream is paused anyway
                    // so `Continue` is also okay
                    //return StreamingState::Continue;
                    return StreamingState::Pause;
                }
                log::warn!("St resumed at index 5");
                return StreamingState::Resume;
            } else if !self.loc {
                self.loc = true;
                log::warn!("St paused at index 5");
                return StreamingState::Pause;
            }
        }

        StreamingState::Continue
    }

    fn action(&mut self, msg: Num, _: &mut Context<Self>) {
        self.sum += msg.0;
        self.counter += 1;
        log::info!("current sum: {}", self.sum);
    }

    fn finished(&mut self, _: &mut Context<Self>) {
        log::info!("stream finished, whole sum: {}", self.sum);
    }
}

#[test]
#[cfg(feature = "tokio")]
fn test_stream() {
    use crossbus::rt::{runtime_tokio::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        //common::init_module_level("test_stream", log::Level::Debug);
        common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
    #[crossbus::main(runtime = tokio)]
    async fn main() {
        let (addr, _) = Sum::start();
    }
    main();
}

#[test]
#[cfg(feature = "async-std")]
fn test_stream() {
    use crossbus::rt::{runtime_async_std::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        //common::init_module_level("test_stream", log::Level::Debug);
        common::init();
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
    #[crossbus::main(runtime = async-std)]
    async fn main() {
        let (addr, _) = Sum::start();
    }
    main();
}

#[cfg(any(feature = "wasm32"))]
#[wasm_bindgen_test::wasm_bindgen_test]
fn test_stream() {
    use crossbus::rt::{runtime_wasm32::Runtime, Spawning};

    // each test module only run it ONCE
    if !LOGGER_INIT.load(Ordering::SeqCst) {
        common::init_module_level("test_stream", log::Level::Debug);
        LOGGER_INIT.store(true, Ordering::SeqCst);
    }
    #[crossbus::main(runtime = wasm32)]
    async fn main() {
        let (addr, _) = Sum::start();
    }
    main();
}

#[cfg(any(feature = "wasm32"))]
fn now() -> f64 {
    use js_sys::Date;
    //log::info!("new date: {:?}", Date::new_0());

    Date::now() / 1000.0
}

fn get_now() -> f64 {
    #[cfg(any(feature = "tokio", feature = "async-std"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
    #[cfg(feature = "wasm32")]
    {
        now()
    }
    #[cfg(not(any(feature = "wasm32", feature = "tokio", feature = "async-std")))]
    unimplemented!()
}
