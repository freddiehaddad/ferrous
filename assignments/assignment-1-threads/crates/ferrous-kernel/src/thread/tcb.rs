use crate::types::ThreadHandle;
use alloc::vec::Vec;
use ferrous_vm::{Cpu, PrivilegeMode, VirtAddr};

/// Represents the current state of a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is ready to run and is in the scheduler's queue.
    Ready,
    /// Thread is currently executing on the CPU.
    Running,
    /// Thread is blocked (e.g., waiting for I/O or a lock).
    Blocked,
    /// Thread is waiting for another thread to exit (waitpid).
    Waiting { target: ThreadHandle },
    /// Thread has finished execution but hasn't been cleaned up yet.
    Terminated { exit_code: i32 },
}

/// Represents an open file descriptor for a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDescriptor {
    File {
        inode_id: u32,
        offset: u32,
        flags: u32,
    },
    Pipe {
        pipe_id: u32,
        is_write: bool,
    },
}

/// Thread Control Block (TCB)
///
/// This structure holds all the information necessary to manage a thread's
/// execution. This includes its unique ID (handle), current state,
/// saved CPU context (registers), memory management info, and open files.
pub struct ThreadControlBlock {
    pub handle: ThreadHandle,
    pub state: ThreadState,
    pub context: SavedContext,
    pub stack_pointer: u32,
    pub kernel_stack: u32, // For kernel stack if needed
    pub program_break: u32,
    pub file_descriptors: Vec<Option<FileDescriptor>>,
}

/// Helper struct to save and restore CPU state during context switches.
#[derive(Debug, Clone, Copy)]
pub struct SavedContext {
    pub pc: u32,
    pub regs: [u32; 32],
    pub satp: u32,
    pub mode: PrivilegeMode,
}

impl SavedContext {
    /// Creates a new context for a fresh thread.
    ///
    /// Sets the Program Counter (PC) to the entry point and the Stack Pointer (SP)
    /// to the top of the allocated stack.
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

    /// Saves the current CPU state into this struct.
    ///
    /// This is called when a thread is being preempted or yields. We must save
    /// all general-purpose registers, the PC, and the current privilege mode so
    /// we can resume exactly where we left off.
    ///
    /// # Assignment 1
    /// Implement the logic to copy CPU registers to the `SavedContext` struct.
    pub fn save_from(&mut self, cpu: &Cpu) {
        // TODO: Assignment 1 - Implement context saving
        todo!("Assignment 1: save_from");
    }

    /// Restores the CPU state from this struct.
    ///
    /// This is called when the scheduler selects this thread to run. We load
    /// the saved registers back into the CPU.
    ///
    /// # Assignment 1
    /// Implement the logic to copy values from `SavedContext` back to the CPU.
    pub fn restore_to(&self, cpu: &mut Cpu) {
        // TODO: Assignment 1 - Implement context restoration
        todo!("Assignment 1: restore_to");
    }

    /// Helper to modify a specific register in the saved context (e.g., for return values).
    pub fn write_reg(&mut self, reg: ferrous_vm::Register, val: u32) {
        if reg.val() > 0 && reg.val() < 32 {
            self.regs[reg.val()] = val;
        }
    }
}
