use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use ferrous_vm::{Cpu, Memory, PhysAddr, TrapError, VirtAddr};
use goblin::elf;
use log::{info, warn};

use crate::error::SyscallError;
use crate::fs::FileSystem;
use crate::memory::{self, copy_from_user, copy_to_user, translate_vaddr};
use crate::syscall::{Syscall, SyscallReturn};
use crate::thread::ThreadManager;

pub fn handle_syscall(
    syscall: Syscall,
    thread_manager: &mut ThreadManager,
    file_system: &mut Option<FileSystem>,
    memory: &mut dyn Memory,
    cpu: &mut Cpu,
) -> Result<VirtAddr, TrapError> {
    match syscall {
        Syscall::Fork => {
            // TODO: Assignment 4 - Implement Fork
            // 1. Duplicate current thread's TCB.
            // 2. Duplicate address space (Copy-on-Write optional).
            // 3. Return 0 to child, child PID to parent.
            todo!("Assignment 4: Fork");
        }
        Syscall::WaitPid { pid } => {
            // TODO: Assignment 4 - Implement WaitPid
            // 1. Check if pid is a valid child.
            // 2. Block until child exits.
            // 3. Return exit code.
            todo!("Assignment 4: WaitPid");
        }
        Syscall::Exec {
            path_ptr,
            path_len,
            args_ptr,
            args_len,
        } => {
            // TODO: Assignment 4 - Implement Exec
            // 1. Read ELF file from disk.
            // 2. Parse ELF.
            // 3. Create new Address Space.
            // 4. Load Segments.
            // 5. Setup Stack.
            // 6. Create Thread/Process (or replace current).
            todo!("Assignment 4: Exec");
        }
        _ => Err(TrapError::HandlerPanic("Process: Unhandled syscall".into())),
    }
}
