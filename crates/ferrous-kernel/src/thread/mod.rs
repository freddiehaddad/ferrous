pub mod scheduler;
pub mod tcb;

use crate::types::ThreadHandle;
use ferrous_vm::{Cpu, Memory, Register, VirtAddr};
use scheduler::{RoundRobinScheduler, Scheduler};
use std::collections::HashMap;
use tcb::{ThreadControlBlock, ThreadState};

pub struct ThreadManager {
    pub threads: HashMap<ThreadHandle, ThreadControlBlock>,
    pub scheduler: Box<dyn Scheduler>,
    pub current_thread: Option<ThreadHandle>,
    pub next_handle: u32,
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            threads: HashMap::new(),
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
                context: tcb::SavedContext::new(VirtAddr::new(cpu.pc), cpu.regs[2], cpu.satp),
                stack_pointer: cpu.regs[2],
                kernel_stack: 0,
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

        // Inherit SATP from current thread or kernel default?
        // Since we are creating a thread in the SAME process (conceptually for now),
        // we share the address space (SATP).
        // If current_thread is set, use its SATP.
        let satp = if let Some(current) = self.current_thread {
            if let Some(parent) = self.threads.get(&current) {
                parent.context.satp
            } else {
                0 // Should not happen
            }
        } else {
            0 // Kernel/Boot SATP (should be set by now if ensure_current_thread called)
        };

        let tcb = ThreadControlBlock {
            handle,
            state: ThreadState::Ready,
            context: tcb::SavedContext::new(entry_point, stack_top, satp),
            stack_pointer: stack_top,
            kernel_stack: 0, // Assume no kernel stack switch for now (running in user mode usually)
        };

        self.threads.insert(handle, tcb);
        self.scheduler.enqueue(handle);

        Ok(handle)
    }

    pub fn yield_thread(&mut self, cpu: &mut Cpu) {
        if let Some(current) = self.current_thread {
            // Save context
            if let Some(tcb) = self.threads.get_mut(&current) {
                // If the thread is Blocked, it was set by block_current_thread and shouldn't be set to Ready.
                // We only set to Ready if it was Running.
                if tcb.state == ThreadState::Running {
                    tcb.state = ThreadState::Ready;
                    self.scheduler.enqueue(current);
                }
                tcb.context.save_from(cpu);
            }
        }

        // Pick next thread
        if let Some(next) = self.scheduler.schedule() {
            self.current_thread = Some(next);
            if let Some(tcb) = self.threads.get_mut(&next) {
                tcb.state = ThreadState::Running;
                tcb.context.restore_to(cpu);
            }
        } else {
            // No threads ready? Should verify if main thread exits.
            // If idle, maybe panic or halt?
            // Assuming at least one thread (main) exists initially.
        }
    }

    pub fn exit_current_thread(&mut self, code: i32) {
        if let Some(current) = self.current_thread {
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

    pub fn wake_thread(&mut self, handle: ThreadHandle) {
        if let Some(tcb) = self.threads.get_mut(&handle) {
            if tcb.state == ThreadState::Blocked {
                tcb.state = ThreadState::Ready;
                self.scheduler.enqueue(handle);
            }
        }
    }
}
