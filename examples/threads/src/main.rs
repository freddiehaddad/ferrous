#![no_std]
#![no_main]

use core::panic::PanicInfo;
use ferrous_user::{exit, print, println, spawn, yield_now};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Main thread started");

    static mut STACK: [u8; 4096] = [0; 4096];

    unsafe {
        let stack_top = STACK.as_mut_ptr().add(STACK.len());
        spawn(thread_func, stack_top);
    }

    println!("Thread 1: Loop start");
    for i in 0..3 {
        println!("Thread 1: Iteration {}", i);
        yield_now();
    }
    println!("Thread 1: Loop end");

    exit(0)
}

extern "C" fn thread_func() {
    println!("Thread 2: Loop start");
    for i in 0..3 {
        println!("Thread 2: Iteration {}", i);
        yield_now();
    }
    println!("Thread 2: Loop end");

    exit(0)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
