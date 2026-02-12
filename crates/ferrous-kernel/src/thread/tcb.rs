use crate::types::ThreadHandle;
use ferrous_vm::{Cpu, Register, VirtAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked, // For later synchronization
    Terminated { exit_code: i32 },
}

pub struct ThreadControlBlock {
    pub handle: ThreadHandle,
    pub state: ThreadState,
    pub context: SavedContext,
    pub stack_pointer: u32,
    pub kernel_stack: u32, // For kernel stack if needed
}

#[derive(Debug, Clone, Copy)]
pub struct SavedContext {
    pub pc: u32,
    pub regs: [u32; 32],
}

impl SavedContext {
    pub fn new(entry_point: VirtAddr, stack_top: u32) -> Self {
        let mut regs = [0; 32];
        regs[2] = stack_top; // SP
        Self {
            pc: entry_point.val(),
            regs,
        }
    }

    pub fn save_from(&mut self, cpu: &Cpu) {
        self.pc = cpu.pc;
        self.regs = cpu.regs;
    }

    pub fn restore_to(&self, cpu: &mut Cpu) {
        cpu.pc = self.pc;
        cpu.regs = self.regs;
    }
}
