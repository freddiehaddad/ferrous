//! Ferrous RISC-V Virtual Machine
//!
//! This crate implements a RISC-V RV32IMA interpreter that simulates
//! the hardware environment for the Ferrous operating system.

pub mod cpu;
pub mod error;
pub mod instruction;
pub mod memory;
pub mod trap;

pub use error::VmError;
pub use instruction::Instruction;
pub use memory::{PhysAddr, VirtAddr};
pub use trap::{TrapCause, TrapHandler};

/// Configuration for the virtual machine
pub struct VmConfig {
    /// Physical memory size in bytes
    pub memory_size: usize,
    /// Enable MMU (virtual memory)
    pub enable_mmu: bool,
    /// Enable timer device
    pub enable_timer: bool,
    /// Timer interval in milliseconds
    pub timer_interval_ms: u64,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            memory_size: 128 * 1024 * 1024, // 128 MB
            enable_mmu: false,              // Start simple
            enable_timer: false,
            timer_interval_ms: 10,
        }
    }
}

/// Main virtual machine structure
pub struct VirtualMachine {
    cpu: Cpu,
    memory: Memory,
    trap_handler: Box<dyn TrapHandler>,
}

impl VirtualMachine {
    pub fn new(config: VmConfig, trap_handler: Box<dyn TrapHandler>) -> Result<Self, VmError> {
        todo!("Implement VM initialization")
    }

    pub fn step(&mut self) -> Result<(), VmError> {
        todo!("Implement single instruction execution")
    }

    pub fn run(&mut self) -> Result<(), VmError> {
        loop {
            self.step()?;
        }
    }
}
