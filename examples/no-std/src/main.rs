#![no_std]
#![no_main]

#[global_allocator]
static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();

extern crate alloc;

mod block_on;
mod fibonacci;
mod ping;
mod ring;

use panic_halt as _;

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};

#[entry]
fn main() -> ! {
    block_on::block_on(async {
        fibonacci::run().await;
        hprintln!("=========== fibonacci sequence runs successfully ==========\n").unwrap();
        ping::run().await;
        hprintln!("=========== ping runs successfully ==========\n").unwrap();
        hprintln!(
            "loop a message in a 6 actor-linked cycle for 7 rounds
the message sending order is:
n -> n+1 -> n+2 -> n+3 -> n+4 -> n+5 -> n
the message will be dropped once it is done 
===========================================\n"
        )
        .unwrap();
        ring::run().await;
        hprintln!("=========== bare metal run successfully and exit ==========\n").unwrap();
    });

    // exit QEMU
    // NOTE do not run this on hardware; it can corrupt OpenOCD state
    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}
