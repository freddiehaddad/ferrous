#![no_std]
#![no_main]

#[cfg(not(test))]
use core::panic::PanicInfo;
use ferrous_user::{exit, net, println, spawn, yield_now};

static mut SOCKET_FD: i32 = -1;
static mut RUNNING: bool = true;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();

    println!("Net Test: Starting...");

    // 1. Create Socket
    let fd = match net::socket() {
        Ok(fd) => {
            println!("Socket created: {}", fd);
            fd
        }
        Err(e) => {
            println!("Socket failed: {}", e);
            exit(1);
        }
    };

    unsafe {
        SOCKET_FD = fd;
    }

    // 2. Bind to 0.0.0.0:5555
    let bind_addr = net::SockAddrIn::new(5555, 0); // 0.0.0.0
    match net::bind(fd, &bind_addr) {
        Ok(_) => println!("Bind successful to port 5555"),
        Err(e) => {
            println!("Bind failed: {}", e);
            exit(1);
        }
    }

    // 3. Spawn Receiver Thread
    println!("Spawning receiver thread...");
    spawn(receiver_thread);

    // 4. Sender Loop (Main Thread)
    #[allow(clippy::identity_op)]
    let dest_ip = (10 << 24) | (0 << 16) | (2 << 8) | 2; // 10.0.2.2 (Host)
    let dest_addr = net::SockAddrIn::new(5555, dest_ip);
    let msg = b"Hello from Ferrous Multitasking!";

    let mut counter = 0;
    loop {
        match net::sendto(fd, msg, &dest_addr) {
            Ok(len) => println!("[Sender] Sent packet {} ({} bytes)", counter, len),
            Err(e) => println!("[Sender] Send failed: {}", e),
        }
        counter += 1;

        // Longer delay to allow receiver to process
        // for _ in 0..10 {
        //     yield_now();
        // }

        if counter >= 1 {
            println!("Test limit reached, exiting.");
            unsafe { RUNNING = false };
            // Give receiver a chance to see the flag
            yield_now();
            yield_now();
            exit(0);
        }
    }
}

extern "C" fn receiver_thread() {
    let fd = unsafe { SOCKET_FD };
    let mut buf = [0u8; 128];

    println!("[Receiver] Thread started, listening...");

    while unsafe { RUNNING } {
        match net::recvfrom(fd, &mut buf) {
            Ok((len, src)) => {
                let received = core::str::from_utf8(&buf[..len]).unwrap_or("<invalid utf8>");
                println!(
                    "[Receiver] Got {} bytes from {:x}:{:x}: {}",
                    len, src.addr, src.port, received
                );
            }
            Err(-1) => {
                yield_now();
            }
            Err(e) => {
                println!("[Receiver] Error: {}", e);
                // Don't exit, just retry
                yield_now();
            }
        }
    }
    println!("[Receiver] Stopping...");
    exit(0);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {:?}", info);
    loop {}
}
