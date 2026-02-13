use thiserror::Error;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("initialization error: {0}")]
    Init(String),

    #[error("memory initialization error: {0}")]
    InitializationError(String),
}

#[derive(Debug, Error)]
pub enum SyscallError {
    #[error("invalid syscall number: {0}")]
    InvalidSyscallNumber(u32),

    #[error("invalid argument")]
    InvalidArgument,
}
