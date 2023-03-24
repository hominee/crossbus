extern crate alloc;

use cortex_m_semihosting::hprintln;
/*
 *use linked_list_allocator::LockedHeap;
 *#[global_allocator]
 *static ALLOCATOR: LockedHeap = LockedHeap::empty();
 */

use alloc::string::String;
use crossbus::prelude::*;

struct Summer {
    item: u128,
}

#[derive(Debug, Message)]
struct Fibo(u32);

impl Actor for Summer {
    type Message = Fibo;

    fn create(_ctx: &mut Context<Self>) -> Self {
        hprintln!("actor created").unwrap();
        Self { item: 0 }
    }

    fn action(&mut self, msg: Self::Message, _ctx: &mut Context<Self>) {
        match fibo(msg.0 as _) {
            Ok(item) => {
                hprintln!("fibonacci sequence index: {}", msg.0).unwrap();
                self.item = item;
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        hprintln!("actor stopped, the item is {}", self.item).unwrap();
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

pub async fn run() {
    async fn run() {
        //simple_logger::init_with_level(log::Level::Debug).unwrap();
        let (addr, _) = Summer::start();
        let sender = addr.sender();
        sender.send(Fibo(7)).unwrap();
    }
    run().await;
    crossbus::reactor::Reactor::as_future().await;
}
