#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod vga_buffer;

#[global_allocator]
static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello World with {}", "CrossBus!");

    // still works
    println!("the 2nd Hello World with {}", "CrossBus!");
    loop {}
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
