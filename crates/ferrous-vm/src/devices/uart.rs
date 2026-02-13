use crate::devices::Device;
use crate::error::DeviceError;
use std::io::{self, Write};

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x100;

// Registers offsets
pub const RBR: u32 = 0x00; // Receiver Buffer Register (Read Only)
pub const THR: u32 = 0x00; // Transmitter Holding Register (Write Only)

pub struct UartDevice {
    // We could buffer output here if needed
}

impl UartDevice {
    pub fn new() -> Self {
        Self {}
    }
}

impl Device for UartDevice {
    fn name(&self) -> &str {
        "UART0"
    }

    fn read(&mut self, offset: u32) -> Result<u32, DeviceError> {
        match offset {
            RBR => Ok(0), // No input for now
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u32, value: u32) -> Result<(), DeviceError> {
        match offset {
            THR => {
                let byte = (value & 0xFF) as u8;
                print!("{}", byte as char);
                io::stdout()
                    .flush()
                    .map_err(|e| DeviceError::Io(e.to_string()))?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn tick(&mut self) -> Result<Option<crate::devices::DeviceInterrupt>, DeviceError> {
        Ok(None)
    }
}
