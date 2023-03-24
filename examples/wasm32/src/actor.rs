use crossbus::prelude::*;
use wasm_bindgen::JsCast;

struct Summer;

#[derive(Debug, Message)]
enum Msg {
    Ping(u32),
    Fibo(u32),
}

impl Actor for Summer {
    type Message = Msg;

    fn create(_ctx: &mut Context<Self>) -> Self {
        Self {}
    }

    fn action(&mut self, msg: Self::Message, _ctx: &mut Context<Self>) {
        match msg {
            Msg::Ping(num) => {
                let s = match ping_add(num as _) {
                    Ok(s) => format!("{:?}", s),
                    Err(e) => format!("{:?}", e),
                };
                let window = web_sys::window().expect("Window Access Error");
                let document = window.document().expect("Document Access Error");
                document
                    .get_element_by_id("ping-result")
                    .map(|en| {
                        en.dyn_into::<web_sys::HtmlTextAreaElement>()
                            .expect("TextArea Not Found")
                            .set_value(&s);
                    })
                    .unwrap();
            }
            Msg::Fibo(num) => {
                let s = match fibo(num as _) {
                    Ok(s) => format!("{:?}", s),
                    Err(e) => format!("{:?}", e),
                };
                let window = web_sys::window().expect("Window Access Error");
                let document = window.document().expect("Document Access Error");
                document.get_element_by_id("fibonacci-result").map(|en| {
                    en.dyn_into::<web_sys::HtmlTextAreaElement>()
                        .expect("TextArea Not Found")
                        .set_value(&s);
                });
            }
        }
    }
}

const POW: u32 = 7;
fn ping_add(num: u128) -> Result<u128, String> {
    let mut sum = 0u128;
    for ind in 1..=num {
        let (pow, overflow) = ind.overflowing_pow(POW);
        if overflow {
            return Err("Digit Overflow".into());
        }
        let (delta, overflow) = sum.overflowing_add(pow);
        if overflow {
            return Err("Digit Overflow".into());
        }
        sum = delta;
    }
    Ok(sum)
}

fn fibo(num: u32) -> Result<u128, String> {
    if num <= 2 {
        return Ok(1);
    }
    let mut prev = 1u128;
    let mut next = 1u128;
    for _ in 3..=num as usize {
        let (term, overflow) = prev.overflowing_add(next);
        if overflow {
            return Err("Digit Overflow".into());
        }
        prev = next;
        next = term;
    }
    Ok(next)
}

pub async fn run(num: u32, typ: &'static str) {
    async fn run(num: u32, typ: &'static str) {
        let (addr, _) = Summer::start();
        let sender = addr.sender();
        match typ {
            "ping" => {
                sender.send(Msg::Ping(num)).unwrap();
            }
            "fibonacci" => {
                sender.send(Msg::Fibo(num)).unwrap();
            }
            _ => log::error!("Invalid Message Type"),
        }
    }
    run(num, typ).await;
    crossbus::reactor::Reactor::as_future().await;
}
