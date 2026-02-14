use crate::devices::{Device, DeviceInterrupt};
use crate::error::DeviceError;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub const BLOCK_DEVICE_BASE: u32 = 0x2000_0000;
pub const BLOCK_DEVICE_SIZE: u32 = 0x1000; // 4KB (enough for buffer)

// Register Offsets
const REG_STATUS: u32 = 0x00; // Read-only: 0=Ready, 1=Busy
const REG_COMMAND: u32 = 0x04; // Write-only: 1=Read, 2=Write
const REG_SECTOR: u32 = 0x08; // Sector number to access
                              // const REG_BUFFER: u32 = 0x0C; // Pointer to memory buffer (Physical Address) - Unused in PIO mode
                              // const REG_DATA: u32 = 0x10; // Data port - Unused

// Note: To implement DMA (Direct Memory Access), the Device needs access to System RAM.
// However, our current Device trait structure only allows read/write to the DEVICE registers.
// The CPU drives the VM.
//
// We use a "Buffer" inside the device (PIO / Shared Memory Window).
// 1. Write Sector
// 2. Write Command (Read from Disk to Internal Buffer)
// 3. CPU reads from a mapped memory window (e.g. 0x2000_0100 - 0x2000_0300) which IS the sector buffer.

const SECTOR_SIZE: usize = 512;

pub struct SimpleBlockDevice {
    file: File,
    sector: u32,
    buffer: [u8; SECTOR_SIZE],
}

impl SimpleBlockDevice {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        Ok(Self {
            file,
            sector: 0,
            buffer: [0; SECTOR_SIZE],
        })
    }
}

impl Device for SimpleBlockDevice {
    fn read(&mut self, offset: u32) -> Result<u32, DeviceError> {
        if offset >= 0x100 && offset < 0x100 + SECTOR_SIZE as u32 {
            // Read from internal buffer
            let idx = (offset - 0x100) as usize;
            if idx + 4 > SECTOR_SIZE {
                return Err(DeviceError::InvalidOffset(offset));
            }
            let val = u32::from_le_bytes([
                self.buffer[idx],
                self.buffer[idx + 1],
                self.buffer[idx + 2],
                self.buffer[idx + 3],
            ]);
            return Ok(val);
        }

        match offset {
            REG_STATUS => Ok(0), // Always ready for now
            REG_SECTOR => Ok(self.sector),
            _ => Ok(0),
        }
    }

    fn write(&mut self, offset: u32, val: u32) -> Result<(), DeviceError> {
        if offset >= 0x100 && offset < 0x100 + SECTOR_SIZE as u32 {
            // Write to internal buffer
            let idx = (offset - 0x100) as usize;
            if idx + 4 > SECTOR_SIZE {
                return Err(DeviceError::InvalidOffset(offset));
            }
            let bytes = val.to_le_bytes();
            self.buffer[idx] = bytes[0];
            self.buffer[idx + 1] = bytes[1];
            self.buffer[idx + 2] = bytes[2];
            self.buffer[idx + 3] = bytes[3];
            return Ok(());
        }

        match offset {
            REG_SECTOR => {
                self.sector = val;
                Ok(())
            }
            REG_COMMAND => {
                match val {
                    1 => {
                        // Read from Disk to Buffer
                        let pos = (self.sector as u64) * (SECTOR_SIZE as u64);
                        if self.file.seek(SeekFrom::Start(pos)).is_err() {
                            // Only error if seek fails hard, else assume 0s or similar?
                            // For simplicity, do nothing or log
                        }
                        let _ = self.file.read_exact(&mut self.buffer); // Ignore EOF errors (partial read)
                        Ok(())
                    }
                    2 => {
                        // Write from Buffer to Disk
                        let pos = (self.sector as u64) * (SECTOR_SIZE as u64);
                        let _ = self.file.seek(SeekFrom::Start(pos));
                        let _ = self.file.write_all(&self.buffer);
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn name(&self) -> &str {
        "virtio-block-simple"
    }

    fn tick(&mut self) -> Result<Option<DeviceInterrupt>, DeviceError> {
        Ok(None)
    }
}
