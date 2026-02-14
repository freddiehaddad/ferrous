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
use log::info;
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

        // Inherit SATP from current thread or kernel default?
        // Since we are creating a thread in the SAME process (conceptually for now),
        // we share the address space (SATP).
        // If current_thread is set, use its SATP.
        let (satp, program_break) = if let Some(current) = self.current_thread {
            if let Some(parent) = self.threads.get(&current) {
                (parent.context.satp, parent.program_break)
            } else {
                (0, 0x8040_0000)
            }
        } else {
            (0, 0x8040_0000)
        };

        // FORCE User Mode for new threads created via syscall
        // (If created by kernel internal logic, might be different, but for now Syscall::ThreadCreate implies User)
        let mode = PrivilegeMode::User;

        // Inherit File Descriptors from parent thread
        let file_descriptors = if let Some(current) = self.current_thread {
            if let Some(parent) = self.threads.get(&current) {
                parent.file_descriptors.clone()
            } else {
                vec![None, None, None]
            }
        } else {
            vec![None, None, None]
        };

        let tcb = ThreadControlBlock {
            handle,
            state: ThreadState::Ready,
            context: tcb::SavedContext::new(entry_point, stack_top, satp, mode),
            stack_pointer: stack_top,
            kernel_stack: 0, // Assume no kernel stack switch for now (running in user mode usually)
            program_break,
            file_descriptors,
        };

        self.threads.insert(handle, tcb);
        self.scheduler.enqueue(handle);

        Ok(handle)
    }

    pub fn yield_thread(&mut self, cpu: &mut Cpu) -> bool {
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
            info!("Scheduler: Switched to thread {:?}", next);
            self.current_thread = Some(next);
            if let Some(tcb) = self.threads.get_mut(&next) {
                tcb.state = ThreadState::Running;
                tcb.context.restore_to(cpu);
            }
            true
        } else {
            info!(
                "Scheduler: No ready threads found. Current thread is {:?}",
                self.current_thread
            );

            // If current thread is None (Exited), we have no thread to run -> return false.
            if self.current_thread.is_none() {
                return false;
            }

            // If current thread exists but is Blocked, we CANNOT run it -> return false.
            if let Some(current) = self.current_thread {
                if let Some(tcb) = self.threads.get(&current) {
                    if tcb.state != ThreadState::Running {
                        info!(
                            "Scheduler: Current thread {:?} is {:?}, cannot resume.",
                            current, tcb.state
                        );
                        return false;
                    }
                }
            }

            // If current thread is Running (and was the only one), we continue running it -> return true.
            true
        }
    }

    pub fn exit_current_thread(&mut self, code: i32) {
        info!("ThreadManager: Exiting thread {:?}", self.current_thread);
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
