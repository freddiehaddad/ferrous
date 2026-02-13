pub mod scheduler;
pub mod syscalls;
pub mod tcb;

use crate::types::ThreadHandle;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use ferrous_vm::{Cpu, PrivilegeMode, VirtAddr};
use scheduler::{RoundRobinScheduler, Scheduler};
use tcb::{ThreadControlBlock, ThreadState};

pub struct ThreadManager {
    pub threads: BTreeMap<ThreadHandle, ThreadControlBlock>,
    pub scheduler: Box<dyn Scheduler>,
    pub current_thread: Option<ThreadHandle>,
    pub next_handle: u32,
}

impl Default for ThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            scheduler: Box::new(RoundRobinScheduler::new()),
            current_thread: None,
            next_handle: 1,
        }
    }

    pub fn ensure_current_thread(&mut self, cpu: &Cpu) {
        if self.current_thread.is_none() {
            // Main thread gets handle 1
            let handle = ThreadHandle::new(self.next_handle).unwrap();
            self.next_handle += 1;

            let tcb = ThreadControlBlock {
                handle,
                state: ThreadState::Running,
                context: tcb::SavedContext::new(
                    VirtAddr::new(cpu.pc),
                    cpu.regs[2],
                    cpu.satp,
                    cpu.mode,
                ),
                stack_pointer: cpu.regs[2],
                kernel_stack: 0,
                program_break: 0x8040_0000, // Default heap start (4MB mark)
                file_descriptors: vec![None, None, None], // Reserve stdin, stdout, stderr
            };
            self.threads.insert(handle, tcb);
            self.current_thread = Some(handle);
            // Don't enqueue main thread yet, it's running
        }
    }

    pub fn create_thread(
        &mut self,
        entry_point: VirtAddr,
        stack_top: u32, // User needs to provide stack
    ) -> Result<ThreadHandle, String> {
        let handle = ThreadHandle::new(self.next_handle).unwrap();
        self.next_handle += 1;

        // TODO: Assignment 1
        // 1. Inherit SATP from current thread (or default to 0 for now)
        // 2. Create a new ThreadControlBlock
        //    - State: Ready
        //    - Mode: User
        //    - Stack Pointer: stack_top
        // 3. Store the TCB in self.threads
        // 4. Enqueue the new thread handle in the scheduler

        todo!("Assignment 1: create_thread")
    }

    pub fn yield_thread(&mut self, cpu: &mut Cpu) {
        // TODO: Assignment 1
        // 1. If there is a current thread:
        //    - If it's Running, set it to Ready and enqueue it.
        //    - Save its context from the CPU.
        // 2. Ask scheduler for the next thread.
        // 3. If a next thread is found:
        //    - Set current_thread to this new thread.
        //    - Set its state to Running.
        //    - Restore its context to the CPU.

        todo!("Assignment 1: yield_thread")
    }

    pub fn exit_current_thread(&mut self, code: i32) {
        if let Some(current) = self.current_thread {
            // Find anyone waiting on 'current'
            let mut to_wake = Vec::new();
            for (handle, tcb) in self.threads.iter() {
                if let ThreadState::Waiting { target } = tcb.state {
                    if target == current {
                        to_wake.push(*handle);
                    }
                }
            }

            // Wake them up
            for h in to_wake {
                if let Some(tcb) = self.threads.get_mut(&h) {
                    tcb.state = ThreadState::Ready;
                    // Pass exit code to waiter's A0 (register 10)
                    tcb.context.regs[10] = code as u32;
                    self.scheduler.enqueue(h);
                }
            }

            if let Some(tcb) = self.threads.get_mut(&current) {
                tcb.state = ThreadState::Terminated { exit_code: code };
            }
            self.current_thread = None;
            // Schedule next immediately handled by caller or next trap
        }
    }

    pub fn block_current_thread(&mut self) {
        if let Some(current) = self.current_thread {
            if let Some(tcb) = self.threads.get_mut(&current) {
                tcb.state = ThreadState::Blocked;
            }
        }
    }

    pub fn wait_current_thread(&mut self, target: ThreadHandle) -> Result<Option<i32>, String> {
        // If target doesn't exist or is already terminated, return exit code if possible
        if let Some(target_tcb) = self.threads.get(&target) {
            if let ThreadState::Terminated { exit_code } = target_tcb.state {
                return Ok(Some(exit_code));
            }
        } else {
            // Target not found.
            return Err("Target thread not found".into());
        }

        if let Some(current) = self.current_thread {
            if current == target {
                return Err("Cannot wait on self".into());
            }
            if let Some(tcb) = self.threads.get_mut(&current) {
                tcb.state = ThreadState::Waiting { target };
            }
            Ok(None)
        } else {
            Err("No current thread".into())
        }
    }

    pub fn wake_thread(&mut self, handle: ThreadHandle) {
        if let Some(tcb) = self.threads.get_mut(&handle) {
            if tcb.state == ThreadState::Blocked {
                tcb.state = ThreadState::Ready;
                self.scheduler.enqueue(handle);
            }
        }
    }
}
