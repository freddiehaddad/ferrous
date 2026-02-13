use crate::cpu::Cpu;
use crate::error::TrapError;
use crate::memory::Memory;
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
    fn as_any(&mut self) -> &mut dyn core::any::Any;

    /// Handle a trap. Returns the address to resume execution.
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError>;
}
