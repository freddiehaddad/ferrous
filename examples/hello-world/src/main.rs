#![no_std]
#![no_main]

use core::panic::PanicInfo;
use ferrous_user::{exit, print, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello from Ferrous!");
    println!("Iteration 1 Complete.");

    exit(0)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
