#![no_std]
#![no_main]

extern crate ferrous_user;
use ferrous_user::{exit, println};

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();
    println!("Hello from a separate process!");
    exit(0);
}
