pub mod block;
pub mod uart;

use crate::error::DeviceError;

pub struct DeviceInterrupt {
    pub device_name: String,
    pub irq_number: u32,
}

pub trait Device: Send {
    /// Device name (for debugging)
    fn name(&self) -> &str;

    /// Read a 32-bit word from device register
    fn read(&mut self, offset: u32) -> Result<u32, DeviceError>;

    /// Write a 32-bit word to device register
    fn write(&mut self, offset: u32, value: u32) -> Result<(), DeviceError>;

    /// Called on each VM step (for timers, etc.)
    fn tick(&mut self) -> Result<Option<DeviceInterrupt>, DeviceError>;
}

pub struct DeviceManager {
    devices: Vec<DeviceEntry>,
}

struct DeviceEntry {
    base_addr: u32,
    size: u32,
    device: Box<dyn Device>,
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn add_device(&mut self, base_addr: u32, size: u32, device: Box<dyn Device>) {
        self.devices.push(DeviceEntry {
            base_addr,
            size,
            device,
        });
    }

    pub fn read_word(&self, _addr: u32) -> Result<u32, DeviceError> {
        // Need interior mutability if Device::read is &mut self.
        // But SystemBus::read_word is &self.
        // Option 1: Wrap Device in RefCell/Mutex.
        // Option 2: Change Memory::read_word to &mut self (it's often stateful for devices).
        // Let's check Memory trait.
        // Memory::read_word is &self.
        // This is a conflict. MMIO reads CAN have side effects (Clear on Read).
        // So Memory trait should probably be &mut self for reads too?
        // Or Device uses internal mutability.

        // For now, let's look at `Device` trait again. It has `read(&mut self)`.
        // So we MUST have `&mut self` to call it.
        // But `DeviceManager::read_word` is taking `&self`.
        Err(DeviceError::Io(
            "Memory trait requires &self for read, but devices need mutability".into(),
        ))
    }

    pub fn read_word_mut(&mut self, addr: u32) -> Result<u32, DeviceError> {
        for entry in &mut self.devices {
            if addr >= entry.base_addr && addr < entry.base_addr + entry.size {
                return entry.device.read(addr - entry.base_addr);
            }
        }
        Err(DeviceError::InvalidOffset(addr))
    }

    pub fn write_word(&mut self, addr: u32, value: u32) -> Result<(), DeviceError> {
        for entry in &mut self.devices {
            if addr >= entry.base_addr && addr < entry.base_addr + entry.size {
                return entry.device.write(addr - entry.base_addr, value);
            }
        }
        Err(DeviceError::InvalidOffset(addr))
    }
}
