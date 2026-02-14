#![no_std]
#![no_main]

extern crate ferrous_user;

use ferrous_user::net::{self, SockAddrIn};
use ferrous_user::println;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("Panic: {}", info);
    ferrous_user::exit(1);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();
    println!("Net Test Starting...");

    // 1. Create Socket
    // AF_INET = 2, SOCK_DGRAM = 2, IPPROTO_UDP = 17
    let sockfd = match net::socket(2, 2, 17) {
        Ok(fd) => fd,
        Err(e) => {
            println!("socket failed: {}", e);
            ferrous_user::exit(1);
        }
    };
    println!("Socket created: {}", sockfd);

    // 2. Bind to local port 12345
    let local_port = 12345;
    let local_addr = SockAddrIn::new(0, net::htons(local_port)); // 0.0.0.0
    if let Err(e) = net::bind(sockfd, &local_addr) {
        println!("bind failed: {}", e);
        ferrous_user::exit(1);
    }
    println!("Socket bound to 0.0.0.0:{}", local_port);

    // 3. Send to Host (10.0.2.2) port 5555
    let dest_ip = 0x0A000202;
    let dest_port = 5555;
    let dest_addr = SockAddrIn::new(net::htonl(dest_ip), net::htons(dest_port));

    let msg = "Hello from Ferrous!";
    println!("Sending: {}", msg);

    match net::sendto(sockfd, msg.as_bytes(), 0, &dest_addr) {
        Ok(len) => println!("Sent {} bytes", len),
        Err(e) => {
            println!("sendto failed: {}", e);
            ferrous_user::exit(1);
        }
    }

    // 4. Receive response
    println!("Waiting for response...");
    let mut buf = [0u8; 128];
    match net::recvfrom(sockfd, &mut buf, 0) {
        Ok((len, src)) => {
            println!(
                "Received {} bytes from {:x}:{}",
                len,
                net::ntohl(src.addr),
                net::ntohs(src.port)
            );

            // Print buffer content
            ferrous_user::print!("Data: ");
            for i in 0..len {
                let c = buf[i];
                if c >= 32 && c <= 126 {
                    ferrous_user::print!("{}", c as char);
                } else {
                    ferrous_user::print!(".");
                }
            }
            println!("");
        }
        Err(e) => {
            println!("recvfrom failed: {}", e);
            ferrous_user::exit(1);
        }
    }

    println!("Net Test Passed!");
    ferrous_user::exit(0);
}
