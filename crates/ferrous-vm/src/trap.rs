use crate::memory::VirtAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapCause {
    // Exceptions
    InstructionMisaligned { addr: VirtAddr },
    InstructionAccessFault { addr: VirtAddr },
    IllegalInstruction { instruction: u32 },
    Breakpoint,
    LoadAddressMisaligned { addr: VirtAddr },
    LoadAccessFault { addr: VirtAddr },
    StoreAddressMisaligned { addr: VirtAddr },
    StoreAccessFault { addr: VirtAddr },

    // System calls
    EnvironmentCallFromU, // ecall from user mode
    EnvironmentCallFromS, // ecall from supervisor mode

    // Page faults
    InstructionPageFault { addr: VirtAddr },
    LoadPageFault { addr: VirtAddr },
    StorePageFault { addr: VirtAddr },

    // Interrupts
    TimerInterrupt,
    ExternalInterrupt,
}

/// Trait that the kernel implements to handle traps
pub trait TrapHandler: Send {
    /// Handle a trap. Returns the address to resume execution.
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut Memory,
    ) -> Result<VirtAddr, TrapError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TrapError {
    #[error("unhandled trap: {0:?}")]
    Unhandled(TrapCause),

    #[error("trap handler panicked: {0}")]
    HandlerPanic(String),
}
