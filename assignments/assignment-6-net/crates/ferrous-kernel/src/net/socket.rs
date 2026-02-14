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

// Simple demuxer called by recv loop
pub fn process_rx() {
    let mut buffer = [0u8; 2048];
    let mut driver = DRIVER.lock();

    // Process all available packets
    while let Some(len) = driver.poll() {
        if len == 0 {
            break;
        }

        let len = driver.read_packet(&mut buffer);
        let packet = &buffer[..len];

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
        for socket in sockets.sockets.values_mut() {
            if socket.local_port == dest_port {
                socket.rx_queue.push_back(RxPacket {
                    payload: payload.to_vec(),
                    src_ip,
                    src_port,
                });
                break;
            }
        }
    }
}
