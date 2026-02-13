use crate::devices::Device;
use crate::error::DeviceError;
use std::collections::VecDeque;
use std::io::{self, Read, Write};

pub const UART_BASE: u32 = 0x1000_0000;
pub const UART_SIZE: u32 = 0x100;

// Registers offsets
pub const RBR: u32 = 0x00; // Receiver Buffer Register (Read Only)
pub const THR: u32 = 0x00; // Transmitter Holding Register (Write Only)
pub const LSR: u32 = 0x05; // Line Status Register

pub struct UartDevice {
    input_buffer: VecDeque<u8>,
}

impl Default for UartDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl UartDevice {
    pub fn new() -> Self {
        Self {
            input_buffer: VecDeque::new(),
        }
    }
}

impl Device for UartDevice {
    fn name(&self) -> &str {
        "UART0"
    }

    fn read(&mut self, offset: u32) -> Result<u32, DeviceError> {
        match offset {
            RBR => {
                if self.input_buffer.is_empty() {
                    // This is a blocking read simulation.
                    // In a real emulator, we might poll or use a separate thread.
                    // For this simple OS, blocking the VM until input arrives is acceptable.
                    let mut buffer = [0; 256];
                    match io::stdin().read(&mut buffer) {
                        Ok(0) => return Ok(0), // EOF
                        Ok(n) => {
                            for byte in buffer.iter().take(n) {
                                self.input_buffer.push_back(*byte);
                            }
                        }
                        Err(e) => return Err(DeviceError::Io(e.to_string())),
                    }
                }

                if let Some(byte) = self.input_buffer.pop_front() {
                    // Handle CRLF normalization (Windows/Terminal artifact)
                    if byte == 13 {
                        if let Some(&next) = self.input_buffer.front() {
                            if next == 10 {
                                self.input_buffer.pop_front();
                            }
                        }
                        Ok(10) // Return newline
                    } else {
                        Ok(byte as u32)
                    }
                } else {
                    Ok(0)
                }
            }
            LSR => {
                // Bit 0: Data Ready (DR)
                let dr = if !self.input_buffer.is_empty() { 1 } else { 0 };
                // Bit 5: Transmitter Holding Register Empty (THRE) - always 1 (ready)
                let thre = 1 << 5;
                Ok(dr | thre)
            }
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
