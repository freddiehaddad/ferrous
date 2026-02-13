#![no_std]
#![no_main]

#[cfg(not(test))]
use core::panic::PanicInfo;
use ferrous_user::{exit, print, println, syscall};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();

    println!("File System Test: Reading 'hello.txt'");

    match syscall::file_open("hello.txt") {
        Ok(fd) => {
            println!("File opened successfully. FD: {}", fd);

            let mut buffer = [0u8; 128];
            match syscall::file_read(fd, &mut buffer) {
                Ok(bytes) => {
                    println!("Read {} bytes.", bytes);
                    print!("Content: ");
                    for byte in buffer.iter().take(bytes) {
                        syscall::console_write(1, &[*byte]);
                    }
                    println!("");
                }
                Err(_) => {
                    println!("Failed to read file.");
                }
            }

            syscall::file_close(fd);
            println!("File closed.");
        }
        Err(e) => {
            println!("Failed to open file 'hello.txt'. Error Code: {}", e);
        }
    }

    exit(0)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("PANIC in file-read");
    loop {}
}
