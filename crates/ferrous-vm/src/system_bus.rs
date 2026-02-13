use crate::devices::{Device, DeviceManager};
use crate::error::MemoryError;
use crate::memory::{Memory, PhysAddr, SimpleMemory};

pub struct SystemBus {
    ram: SimpleMemory,
    devices: DeviceManager,
}

impl SystemBus {
    pub fn new(memory_size: usize) -> Self {
        Self {
            ram: SimpleMemory::new(memory_size),
            devices: DeviceManager::new(),
        }
    }

    pub fn add_device(&mut self, base_addr: u32, size: u32, device: Box<dyn Device>) {
        self.devices.add_device(base_addr, size, device);
    }

    pub fn load_program(&mut self, addr: PhysAddr, data: &[u8]) -> Result<(), MemoryError> {
        self.ram.load(addr, data)
    }
}

impl Memory for SystemBus {
    fn read_byte(&mut self, addr: PhysAddr) -> Result<u8, MemoryError> {
        if addr.0 >= 0x8000_0000 {
            self.ram.read_byte(addr)
        } else {
            let word = self.devices.read_word_mut(addr.0)?;
            let shift = (addr.0 % 4) * 8;
            Ok(((word >> shift) & 0xFF) as u8)
        }
    }

    fn write_byte(&mut self, addr: PhysAddr, val: u8) -> Result<(), MemoryError> {
        if addr.0 >= 0x8000_0000 {
            self.ram.write_byte(addr, val)
        } else {
            if addr.0 % 4 != 0 {
                return Err(MemoryError::Misaligned {
                    addr: addr.0,
                    alignment: 4,
                });
            }

            self.devices.write_word(addr.0, val as u32)?;
            Ok(())
        }
    }

    fn read_word(&mut self, addr: PhysAddr) -> Result<u32, MemoryError> {
        if addr.0 >= 0x8000_0000 {
            self.ram.read_word(addr)
        } else {
            self.devices.read_word_mut(addr.0).map_err(Into::into)
        }
    }

    fn write_word(&mut self, addr: PhysAddr, val: u32) -> Result<(), MemoryError> {
        if addr.0 >= 0x8000_0000 {
            self.ram.write_word(addr, val)
        } else {
            self.devices.write_word(addr.0, val).map_err(Into::into)
        }
    }
}
