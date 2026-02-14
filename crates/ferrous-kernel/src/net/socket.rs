use crate::net::driver::DRIVER;
use crate::net::ipv4::Ipv4Header;
use crate::net::udp::UdpHeader;
use crate::sync::spinlock::SpinLock;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use zerocopy::FromBytes;

pub struct RxPacket {
    pub payload: Vec<u8>,
    pub src_ip: [u8; 4],
    pub src_port: u16,
}

pub struct Socket {
    pub local_port: u16,
    pub rx_queue: VecDeque<RxPacket>, // Full packet data + metadata
}

pub struct SocketTable {
    sockets: BTreeMap<u32, Socket>,
    next_id: u32,
}

impl Default for SocketTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketTable {
    pub const fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn create_socket(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.sockets.insert(
            id,
            Socket {
                local_port: 0,
                rx_queue: VecDeque::new(),
            },
        );
        id
    }

    pub fn bind(&mut self, id: u32, port: u16) -> bool {
        if let Some(socket) = self.sockets.get_mut(&id) {
            socket.local_port = port;
            true
        } else {
            false
        }
    }

    pub fn get_socket(&mut self, id: u32) -> Option<&mut Socket> {
        self.sockets.get_mut(&id)
    }
}

pub static SOCKETS: SpinLock<SocketTable> = SpinLock::new(SocketTable::new());

use ferrous_vm::Memory;

// Simple demuxer called by recv loop
pub fn process_rx(memory: &mut dyn Memory) {
    let mut buffer = [0u8; 2048];
    // Use scoped lock or manual lock/unlock to minimize contention

    // We can't hold driver lock while processing if we want concurrency,
    // but poll() usually requires exclusive access to device registers.
    // However, we should copy packet out, drop lock, then process.

    loop {
        let len = {
            let mut driver = DRIVER.lock();
            if let Some(l) = driver.poll(memory) {
                if l > 0 {
                    driver.read_packet(memory, &mut buffer)
                } else {
                    0
                }
            } else {
                0
            }
        };

        if len == 0 {
            break;
        }

        let packet = &buffer[..len];
        log::info!("Kernel: Read packet len {}", len);

        // Parse (Assuming Ethernet II -> IPv4 -> UDP)
        // Eth Header = 14 bytes
        if packet.len() < 14 + 20 + 8 {
            continue;
        } // Min size

        let eth_type = u16::from_be_bytes([packet[12], packet[13]]);
        if eth_type != 0x0800 {
            continue;
        } // Not IPv4

        let ip_offset = 14;
        let ip_header = Ipv4Header::read_from(&packet[ip_offset..ip_offset + 20]).unwrap();

        if ip_header.protocol != 17 {
            continue;
        } // Not UDP

        let udp_offset = ip_offset + 20; // Assuming no options
        let udp_header = UdpHeader::read_from(&packet[udp_offset..udp_offset + 8]).unwrap();

        let dest_port = u16::from_be(udp_header.dest_port);
        let src_port = u16::from_be(udp_header.src_port);
        let src_ip = ip_header.src_ip;
        let payload_offset = udp_offset + 8;
        let payload = &packet[payload_offset..]; // Copy rest

        // Find matching socket
        let mut sockets = SOCKETS.lock();
        let mut found = false;
        for socket in sockets.sockets.values_mut() {
            if socket.local_port == dest_port {
                log::info!("Kernel: Matched socket port {}", dest_port);
                socket.rx_queue.push_back(RxPacket {
                    payload: payload.to_vec(),
                    src_ip,
                    src_port,
                });
                found = true;
                break;
            }
        }
        if !found {
            log::info!("Kernel: No socket for port {}", dest_port);
        }
    }
}
