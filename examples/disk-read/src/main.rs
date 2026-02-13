#![no_std]
#![no_main]

use core::panic::PanicInfo;
use ferrous_user::{exit, print, println, syscall};

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
    for i in 0..512 {
        buffer[i] = 0xFF;
    }

    println!("Reading Sector 0...");
    if let Err(e) = syscall::block_read(0, &mut buffer) {
        println!("Error reading sector 0: {}", e);
        exit(1);
    }

    println!("Read success. First 32 bytes:");
    for i in 0..32 {
        print!("{:02x} ", buffer[i]);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();

    exit(0);
}
