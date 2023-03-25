use cortex_m_semihosting::hprintln;
use crossbus::prelude::*;

struct Summer {
    sum: i32,
}

#[derive(Debug, Message)]
struct Ping(i32);

impl Actor for Summer {
    type Message = Ping;

    fn create(_: &mut Context<Self>) -> Self {
        Self { sum: 0 }
    }

    fn action(&mut self, msg: Self::Message, _: &mut Context<Self>) {
        self.sum += msg.0;
        hprintln!("current sum: {}", self.sum).unwrap();
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        assert_eq!(self.sum, 110);
        hprintln!("final sum: {}", self.sum).unwrap();
    }
}

pub async fn run() {
    async fn run() {
        let (addr, _) = Summer::start();
        let sender = addr.sender();
        sender.send(Ping(3)).unwrap();
        sender.send(Ping(7)).unwrap();
        sender.send(Ping(100)).unwrap();
        assert_eq!(sender.message_number(), 3);
    }
    run().await;
    crossbus::reactor::Reactor::as_future().await;
}
