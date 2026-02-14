use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

fn main() -> std::io::Result<()> {
    // Bind to localhost port 5555
    let socket = UdpSocket::bind("127.0.0.1:5555")?;
    println!("UDP Echo Server listening on {}", socket.local_addr()?);

    let mut buf = [0; 1024];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                let msg = &buf[..amt];

                // Print received packet info
                print!("Received {} bytes from {}: ", amt, src);
                if let Ok(s) = std::str::from_utf8(msg) {
                    println!("{:?}", s);
                } else {
                    println!("{:?}", msg);
                }

                // Echo back
                socket.send_to(msg, src)?;
                println!("Echoed {} bytes back to {}", amt, src);
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
