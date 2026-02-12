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
    // Basic implementation for now, will expand later
}
