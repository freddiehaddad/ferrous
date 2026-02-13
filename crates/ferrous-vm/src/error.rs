use alloc::string::String;
use core::fmt;

#[derive(Debug, PartialEq)]
pub enum DeviceError {
    InvalidOffset(u32),
    Io(String),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceError::InvalidOffset(addr) => write!(f, "invalid device offset: {:#x}", addr),
            DeviceError::Io(msg) => write!(f, "device io error: {}", msg),
        }
    }
}

impl core::error::Error for DeviceError {}

#[derive(Debug, PartialEq)]
pub enum MemoryError {
    OutOfBounds(u32),
    ReadOnly(u32),
    Device(DeviceError),
    Misaligned { addr: u32, alignment: u32 },
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::OutOfBounds(addr) => write!(f, "memory access out of bounds: {:#x}", addr),
            MemoryError::ReadOnly(addr) => write!(f, "write to read-only memory: {:#x}", addr),
            MemoryError::Device(e) => write!(f, "device error: {}", e),
            MemoryError::Misaligned { addr, alignment } => {
                write!(
                    f,
                    "misaligned access: addr={:#x}, align={}",
                    addr, alignment
                )
            }
        }
    }
}

impl core::error::Error for MemoryError {}

impl From<DeviceError> for MemoryError {
    fn from(e: DeviceError) -> Self {
        MemoryError::Device(e)
    }
}

#[derive(Debug, PartialEq)]
pub enum TrapError {
    Halt,
    HandlerPanic(String),
    Unhandled(crate::trap::TrapCause),
}

impl fmt::Display for TrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrapError::Halt => write!(f, "machine halted"),
            TrapError::HandlerPanic(msg) => write!(f, "trap handler panic: {}", msg),
            TrapError::Unhandled(cause) => write!(f, "unhandled trap: {:?}", cause),
        }
    }
}

impl core::error::Error for TrapError {}

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidEncoding(u32),
    InvalidOpcode(u32),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidEncoding(inst) => {
                write!(f, "invalid instruction encoding: {:#x}", inst)
            }
            DecodeError::InvalidOpcode(op) => write!(f, "invalid opcode: {:#x}", op),
        }
    }
}

impl core::error::Error for DecodeError {}

#[derive(Debug, PartialEq)]
pub enum VmError {
    Memory(MemoryError),
    Trap(TrapError),
    InvalidInstruction(u32),
    RegisterIndex(u32),
    Decode(DecodeError),
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::Memory(e) => write!(f, "memory error: {}", e),
            VmError::Trap(e) => write!(f, "trap error: {}", e),
            VmError::InvalidInstruction(inst) => write!(f, "invalid instruction: {:#x}", inst),
            VmError::RegisterIndex(idx) => write!(f, "invalid register index: {}", idx),
            VmError::Decode(e) => write!(f, "decode error: {}", e),
        }
    }
}

impl core::error::Error for VmError {}

impl From<MemoryError> for VmError {
    fn from(e: MemoryError) -> Self {
        VmError::Memory(e)
    }
}

impl From<TrapError> for VmError {
    fn from(e: TrapError) -> Self {
        VmError::Trap(e)
    }
}

impl From<DecodeError> for VmError {
    fn from(e: DecodeError) -> Self {
        VmError::Decode(e)
    }
}
