use ferrous_vm::{Memory, PhysAddr};

pub const BLOCK_DEVICE_BASE: u32 = 0x2000_0000;
// Register Offsets
const _REG_STATUS: u32 = 0x00;
const REG_COMMAND: u32 = 0x04;
const REG_SECTOR: u32 = 0x08;
const REG_BUFFER_START: u32 = 0x100;

pub fn read_sector(memory: &mut dyn Memory, sector: u32, buffer: &mut [u8]) -> Result<(), String> {
    if buffer.len() != 512 {
        return Err("Buffer must be 512 bytes".to_string());
    }

    // 1. Write Sector Number
    memory
        .write_word(PhysAddr::new(BLOCK_DEVICE_BASE + REG_SECTOR), sector)
        .map_err(|e| format!("Failed to write sector: {:?}", e))?;

    // 2. Write Command (1 = Read)
    memory
        .write_word(PhysAddr::new(BLOCK_DEVICE_BASE + REG_COMMAND), 1)
        .map_err(|e| format!("Failed to write command: {:?}", e))?;

    // 3. Read Data from Device Buffer
    // In a real device, we might poll status, but SimpleBlockDevice is synchronous.
    for i in (0..512).step_by(4) {
        let val = memory
            .read_word(PhysAddr::new(
                BLOCK_DEVICE_BASE + REG_BUFFER_START + i as u32,
            ))
            .map_err(|e| format!("Failed to read data: {:?}", e))?;

        let bytes = val.to_le_bytes();
        buffer[i] = bytes[0];
        buffer[i + 1] = bytes[1];
        buffer[i + 2] = bytes[2];
        buffer[i + 3] = bytes[3];
    }

    Ok(())
}

pub fn write_sector(memory: &mut dyn Memory, sector: u32, buffer: &[u8]) -> Result<(), String> {
    if buffer.len() != 512 {
        return Err("Buffer must be 512 bytes".to_string());
    }

    // 1. Write Data to Device Buffer
    for i in (0..512).step_by(4) {
        let val = u32::from_le_bytes([buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]]);
        memory
            .write_word(
                PhysAddr::new(BLOCK_DEVICE_BASE + REG_BUFFER_START + i as u32),
                val,
            )
            .map_err(|e| format!("Failed to write data: {:?}", e))?;
    }

    // 2. Write Sector Number
    memory
        .write_word(PhysAddr::new(BLOCK_DEVICE_BASE + REG_SECTOR), sector)
        .map_err(|e| format!("Failed to write sector: {:?}", e))?;

    // 3. Write Command (2 = Write)
    memory
        .write_word(PhysAddr::new(BLOCK_DEVICE_BASE + REG_COMMAND), 2)
        .map_err(|e| format!("Failed to write command: {:?}", e))?;

    Ok(())
}
