use crate::trap::TrapError;

#[derive(Debug, thiserror::Error)]
pub enum VmError {
    #[error("memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("trap error: {0}")]
    Trap(#[from] TrapError),
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("out of bounds access: {0:#x}")]
    OutOfBounds(u32),

    #[error("misaligned access: address {addr:#x}, alignment {alignment}")]
    Misaligned { addr: u32, alignment: u32 },

    #[error("access violation: tried to {op} at {addr:#x}")]
    AccessViolation { op: &'static str, addr: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid opcode: {0:#x}")]
    InvalidOpcode(u32),

    #[error("invalid instruction encoding: {0:#x}")]
    InvalidEncoding(u32),
}
