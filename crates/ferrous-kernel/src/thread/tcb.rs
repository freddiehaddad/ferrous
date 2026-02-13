use crate::types::ThreadHandle;
use ferrous_vm::{Cpu, PrivilegeMode, VirtAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Waiting { target: ThreadHandle },
    Terminated { exit_code: i32 },
}

pub struct FileDescriptor {
    pub inode_id: u32,
    pub offset: u32,
    pub flags: u32,
}

pub struct ThreadControlBlock {
    pub handle: ThreadHandle,
    pub state: ThreadState,
    pub context: SavedContext,
    pub stack_pointer: u32,
    pub kernel_stack: u32, // For kernel stack if needed
    pub program_break: u32,
    pub file_descriptors: Vec<Option<FileDescriptor>>,
}

#[derive(Debug, Clone, Copy)]
pub struct SavedContext {
    pub pc: u32,
    pub regs: [u32; 32],
    pub satp: u32,
    pub mode: PrivilegeMode,
}

impl SavedContext {
    pub fn new(entry_point: VirtAddr, stack_top: u32, satp: u32, mode: PrivilegeMode) -> Self {
        let mut regs = [0; 32];
        regs[2] = stack_top; // SP
        Self {
            pc: entry_point.val(),
            regs,
            satp,
            mode,
        }
    }

    pub fn save_from(&mut self, cpu: &Cpu) {
        self.pc = cpu.pc;
        self.regs = cpu.regs;
        self.satp = cpu.satp;
        self.mode = cpu.mode;
    }

    pub fn restore_to(&self, cpu: &mut Cpu) {
        cpu.pc = self.pc;
        cpu.regs = self.regs;
        cpu.satp = self.satp;
        cpu.mode = self.mode;
    }

    pub fn write_reg(&mut self, reg: ferrous_vm::Register, val: u32) {
        if reg.val() > 0 && reg.val() < 32 {
            self.regs[reg.val()] = val;
        }
    }
}
