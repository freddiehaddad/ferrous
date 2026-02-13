#![no_std]
#![no_main]

#[cfg(not(test))]
use core::panic::PanicInfo;
use ferrous_user::{exit, println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello from Ferrous!");
    println!("Iteration 1 Complete.");

    exit(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
