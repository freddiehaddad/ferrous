use zerocopy::{AsBytes, FromBytes, FromZeroes};

#[derive(Debug, Clone, Copy, FromZeroes, FromBytes, AsBytes)]
#[repr(C)]
pub struct Ipv4Header {
    pub version_ihl: u8, // Version (4 bits) + IHL (4 bits)
    pub tos: u8,
    pub total_length: u16,   // Big Endian
    pub identification: u16, // Big Endian
    pub flags_fragment: u16, // Big Endian
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16, // Big Endian
    pub src_ip: [u8; 4],
    pub dest_ip: [u8; 4],
}

impl Ipv4Header {
    pub fn new(src: [u8; 4], dest: [u8; 4], protocol: u8, len: u16) -> Self {
        Self {
            version_ihl: 0x45, // Version 4, Header Length 5 (20 bytes)
            tos: 0,
            total_length: len.to_be(),
            identification: 0,
            flags_fragment: 0x0040u16.to_be(), // Don't Fragment
            ttl: 64,
            protocol,
            checksum: 0, // Calculated later
            src_ip: src,
            dest_ip: dest,
        }
    }

    pub fn calculate_checksum(&mut self) {
        self.checksum = 0;
        let bytes = self.as_bytes();
        let mut sum = 0u32;

        for i in (0..bytes.len()).step_by(2) {
            let word = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            sum += word as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        self.checksum = (!sum as u16).to_be();
    }
}
