use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug, Clone, Copy, FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
pub struct UdpHeader {
    pub src_port: u16,  // Big Endian
    pub dest_port: u16, // Big Endian
    pub length: u16,    // Big Endian
    pub checksum: u16,  // Big Endian
}

impl UdpHeader {
    pub fn new(src: u16, dest: u16, len: u16) -> Self {
        Self {
            src_port: src.to_be(),
            dest_port: dest.to_be(),
            length: len.to_be(),
            checksum: 0, // Optional in IPv4
        }
    }
}
