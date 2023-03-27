use crossbus::prelude::*;

struct Sum {
    item: u128,
}

#[derive(Debug, Message)]
struct Fibo(u32);

impl Actor for Sum {
    type Message = Fibo;

    fn create(_: &mut Context<Self>) -> Self {
        Self { item: 0 }
    }

    fn action(&mut self, msg: Self::Message, _: &mut Context<Self>) {
        match fibo(msg.0 as _) {
            Ok(item) => {
                self.item = item;
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        assert_eq!(self.item, 13);
    }
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
            return Err("Digit Overflow, input too big".into());
        }
        prev = next;
        next = term;
    }
    Ok(next)
}

//with async std runtime
#[cfg(feature = "async-std")]
#[crossbus::main(runtime = async-std)]
pub async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Sum::start();
    let sender = addr.sender();
    sender.send(Fibo(7)).unwrap();
}

//with tokio runtime
#[cfg(feature = "tokio")]
#[crossbus::main(runtime = tokio)]
pub async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Sum::start();
    let sender = addr.sender();
    sender.send(Fibo(7)).unwrap();
}

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
fn main() {}
