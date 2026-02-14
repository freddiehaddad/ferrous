use alloc::string::String;
use core::fmt;

#[derive(Debug)]
pub enum KernelError {
    Init(String),
    InitializationError(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::Init(s) => write!(f, "initialization error: {}", s),
            KernelError::InitializationError(s) => write!(f, "memory initialization error: {}", s),
        }
    }
}

impl core::error::Error for KernelError {}

#[derive(Debug)]
pub enum SyscallError {
    InvalidSyscallNumber(u32),
    InvalidArgument,
}

impl fmt::Display for SyscallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyscallError::InvalidSyscallNumber(n) => write!(f, "invalid syscall number: {}", n),
            SyscallError::InvalidArgument => write!(f, "invalid argument"),
        }
    }
}

impl core::error::Error for SyscallError {}
