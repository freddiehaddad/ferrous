use crate::sync::spinlock::SpinLock;
use core::ptr;

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

impl NetDriver {
    pub const fn new() -> Self {
        Self {
            base_addr: NET_BASE,
        }
    }

    pub fn poll(&mut self) -> Option<usize> {
        unsafe {
            let status = ptr::read_volatile((self.base_addr + REG_STATUS) as *const u32);
            if status == 1 {
                let len = ptr::read_volatile((self.base_addr + REG_LENGTH) as *const u32);
                Some(len as usize)
            } else {
                None
            }
        }
    }

    pub fn read_packet(&mut self, buffer: &mut [u8]) -> usize {
        unsafe {
            let len = ptr::read_volatile((self.base_addr + REG_LENGTH) as *const u32) as usize;
            // Cap at buffer length
            let read_len = if len > buffer.len() {
                buffer.len()
            } else {
                len
            };

            // Read from MMIO window
            let window_base = (self.base_addr + BUFFER_OFFSET) as *const u32;
            let mut i = 0;
            while i < read_len {
                let word = ptr::read_volatile(window_base.add(i / 4));
                let bytes = word.to_le_bytes();
                for j in 0..4 {
                    if i + j < read_len {
                        buffer[i + j] = bytes[j];
                    }
                }
                i += 4;
            }

            // Acknowledge read (clears buffer)
            ptr::write_volatile((self.base_addr + REG_COMMAND) as *mut u32, 2);

            read_len
        }
    }

    pub fn send_packet(&mut self, data: &[u8]) {
        unsafe {
            let len = data.len();
            if len > 2048 {
                return;
            }

            // Write length
            ptr::write_volatile((self.base_addr + REG_LENGTH) as *mut u32, len as u32);

            // Write to MMIO window
            let window_base = (self.base_addr + BUFFER_OFFSET) as *mut u32;
            let mut i = 0;
            while i < len {
                let mut word_bytes = [0u8; 4];
                for j in 0..4 {
                    if i + j < len {
                        word_bytes[j] = data[i + j];
                    }
                }
                let word = u32::from_le_bytes(word_bytes);
                ptr::write_volatile(window_base.add(i / 4), word);
                i += 4;
            }

            // Command: Send
            ptr::write_volatile((self.base_addr + REG_COMMAND) as *mut u32, 1);
        }
    }
}

pub static DRIVER: SpinLock<NetDriver> = SpinLock::new(NetDriver::new());
