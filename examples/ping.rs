use crossbus::prelude::*;

struct Summer {
    sum: i32,
}

#[derive(Debug, Message)]
struct Ping(i32);

impl Actor for Summer {
    type Message = Ping;

    fn create(_ctx: &mut Context<Self>) -> Self {
        Self { sum: 0 }
    }

    fn action(&mut self, msg: Self::Message, _ctx: &mut Context<Self>) {
        self.sum += msg.0;
        println!("current sum: {}", self.sum);
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        assert_eq!(self.sum, 110);
        println!("final sum: {}", self.sum);
    }
}

// with tokio runtime
#[cfg(feature = "tokio")]
#[crossbus::main(runtime = tokio)]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Summer::start();
    let sender = addr.sender();
    sender.send(Ping(3)).unwrap();
    sender.send(Ping(7)).unwrap();
    sender.send(Ping(100)).unwrap();
    assert_eq!(sender.message_number(), 3);
}

// with async std runtime
#[cfg(feature = "async-std")]
#[crossbus::main(runtime = async-std)]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let (addr, _) = Summer::start();
    let sender = addr.sender();
    sender.send(Ping(3)).unwrap();
    sender.send(Ping(7)).unwrap();
    sender.send(Ping(100)).unwrap();
    assert_eq!(sender.message_number(), 3);
}

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
fn main() {}
