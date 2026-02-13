#![no_std]
#![no_main]

#[cfg(not(test))]
use core::panic::PanicInfo;
use ferrous_user::{exit, print, println, syscall};

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {}", info);
    exit(1);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();

    println!("Disk Read Example");

    let mut buffer = [0u8; 512];

    // Fill buffer with junk to verify overwrite
    buffer.fill(0);

    println!("Reading Sector 0...");
    if let Err(e) = syscall::block_read(0, &mut buffer) {
        println!("Error reading sector 0: {}", e);
        exit(1);
    }

    println!("Read success. First 32 bytes:");
    for (i, byte) in buffer.iter().enumerate().take(32) {
        print!("{:02x} ", byte);
        if (i + 1) % 16 == 0 {
            println!("");
        }
    }
    println!();

    exit(0);
}
