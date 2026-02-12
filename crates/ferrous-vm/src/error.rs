use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum VmError {
    #[error("memory error: {0}")]
    Memory(#[from] MemoryError),

    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),

    #[error("trap error: {0}")]
    Trap(#[from] TrapError),

    #[error("device error: {0}")]
    Device(#[from] DeviceError),
}

#[derive(Debug, Error, PartialEq)]
pub enum MemoryError {
    #[error("out of bounds access: {0:#x}")]
    OutOfBounds(u32),

    #[error("misaligned access: address {addr:#x}, alignment {alignment}")]
    Misaligned { addr: u32, alignment: u32 },

    #[error("access violation: tried to {op} at {addr:#x}")]
    AccessViolation { op: &'static str, addr: u32 },
}

#[derive(Debug, Error, PartialEq)]
pub enum DecodeError {
    #[error("invalid opcode: {0:#x}")]
    InvalidOpcode(u32),

    #[error("invalid instruction encoding: {0:#x}")]
    InvalidEncoding(u32),
}

#[derive(Debug, Error, PartialEq)]
pub enum TrapError {
    #[error("unhandled trap: {0:?}")]
    Unhandled(crate::trap::TrapCause),

    #[error("trap handler panicked: {0}")]
    HandlerPanic(String),

    #[error("system halt")]
    Halt,
}

#[derive(Debug, Error, PartialEq)]
pub enum DeviceError {
    #[error("invalid register offset: {0:#x}")]
    InvalidOffset(u32),

    #[error("device not ready")]
    NotReady,

    #[error("I/O error: {0}")]
    Io(String),
}
