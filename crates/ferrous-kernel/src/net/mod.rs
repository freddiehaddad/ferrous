pub mod driver;
pub mod ipv4;
pub mod socket;
pub mod syscalls;
pub mod udp;

use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, AsBytes, FromZeroes)]
pub struct SockAddrIn {
    pub family: u16,
    pub port: u16, // Big Endian
    pub addr: u32, // Big Endian
    pub zero: [u8; 8],
}

// Common types?
pub type MacAddress = [u8; 6];
pub type Ipv4Address = [u8; 4];
