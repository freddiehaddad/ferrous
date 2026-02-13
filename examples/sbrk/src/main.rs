#![no_std]
#![no_main]

use core::panic::PanicInfo;
use ferrous_user::{exit, println, syscall};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();
    println!("Sbrk Test Started");

    // Initial break
    let start_brk = syscall::sbrk(0);
    println!("Initial break: {:#x}", start_brk);

    // Allocate 4KB (1 page)
    let old_brk = syscall::sbrk(4096);
    println!("Allocated 4KB. Old break: {:#x}", old_brk);

    if old_brk != start_brk {
        println!("Error: return value mismatch");
    }

    let new_brk = syscall::sbrk(0);
    println!("New break: {:#x}", new_brk);

    if new_brk != start_brk + 4096 {
        println!("Error: break did not advance correctly");
    }

    // Write to the new memory
    let ptr = old_brk as *mut u32;
    unsafe {
        *ptr = 0xDEADBEEF;
        println!("Wrote 0xDEADBEEF to {:#x}", old_brk);
        let val = *ptr;
        println!("Read back: {:#x}", val);

        if val != 0xDEADBEEF {
            println!("Error: Memory read mismatch!");
        } else {
            println!("Memory check passed!");
        }
    }

    exit(0);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
