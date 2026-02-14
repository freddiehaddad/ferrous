use crate::sync::spinlock::SpinLock;
use ferrous_vm::{Memory, PhysAddr};

// Base Address
const NET_BASE: u32 = 0x3000_0000;

// Register Offsets
const REG_STATUS: u32 = 0x00;
const REG_COMMAND: u32 = 0x04;
const REG_LENGTH: u32 = 0x08;
const BUFFER_OFFSET: u32 = 0x100;

pub struct NetDriver {
    base_addr: u32,
}

impl Default for NetDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl NetDriver {
    pub const fn new() -> Self {
        Self {
            base_addr: NET_BASE,
        }
    }

    pub fn poll(&mut self, memory: &mut dyn Memory) -> Option<usize> {
        // Read Status
        let status = memory
            .read_word(PhysAddr::new(self.base_addr + REG_STATUS))
            .ok()?;
        if status == 1 {
            // Read Length
            let len = memory
                .read_word(PhysAddr::new(self.base_addr + REG_LENGTH))
                .ok()?;
            Some(len as usize)
        } else {
            None
        }
    }

    pub fn read_packet(&mut self, memory: &mut dyn Memory, buffer: &mut [u8]) -> usize {
        let len = memory
            .read_word(PhysAddr::new(self.base_addr + REG_LENGTH))
            .unwrap_or(0) as usize;

        // Cap at buffer length
        let read_len = if len > buffer.len() {
            buffer.len()
        } else {
            len
        };

        // Read from MMIO window
        let window_base = self.base_addr + BUFFER_OFFSET;
        let mut i = 0;
        while i < read_len {
            if let Ok(word) = memory.read_word(PhysAddr::new(window_base + (i as u32))) {
                let bytes = word.to_le_bytes();
                for j in 0..4 {
                    if i + j < read_len {
                        buffer[i + j] = bytes[j];
                    }
                }
            }
            i += 4;
        }

        // Acknowledge read (clears buffer)
        // Command: Recv (2)
        let _ = memory.write_word(PhysAddr::new(self.base_addr + REG_COMMAND), 2);

        read_len
    }

    pub fn send_packet(&mut self, memory: &mut dyn Memory, data: &[u8]) {
        let len = data.len();
        if len > 2048 {
            return;
        }

        // Write length
        let _ = memory.write_word(PhysAddr::new(self.base_addr + REG_LENGTH), len as u32);

        // Write to MMIO window
        let window_base = self.base_addr + BUFFER_OFFSET;
        let mut i = 0;
        while i < len {
            let mut word_bytes = [0u8; 4];
            for j in 0..4 {
                if i + j < len {
                    word_bytes[j] = data[i + j];
                }
            }
            let word = u32::from_le_bytes(word_bytes);
            let _ = memory.write_word(PhysAddr::new(window_base + (i as u32)), word);
            i += 4;
        }

        // Command: Send (1)
        let _ = memory.write_word(PhysAddr::new(self.base_addr + REG_COMMAND), 1);
    }
}

pub static DRIVER: SpinLock<NetDriver> = SpinLock::new(NetDriver::new());
