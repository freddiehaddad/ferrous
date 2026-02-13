pub mod error;
pub mod sync;
pub mod syscall;
pub mod thread;
pub mod types;

use crate::error::KernelError;
use crate::sync::Mutex;
use ferrous_vm::{Cpu, Memory, TrapCause, TrapError, TrapHandler, VirtAddr};
use log::{debug, info};
use std::collections::HashMap;
use thread::ThreadManager;

pub struct Kernel {
    thread_manager: ThreadManager,
    mutexes: HashMap<u32, Mutex>,
    next_mutex_id: u32,
}

const UART_BASE: u32 = 0x1000_0000;
const UART_THR_OFFSET: u32 = 0x00;

impl Kernel {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {
            thread_manager: ThreadManager::new(),
            mutexes: HashMap::new(),
            next_mutex_id: 1,
        })
    }

    pub fn handle_syscall(
        &mut self,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError> {
        // Decode syscall
        let syscall = syscall::Syscall::from_registers(cpu)
            .map_err(|e| TrapError::HandlerPanic(format!("Syscall decode error: {:?}", e)))?;

        debug!("Syscall: {:?}", syscall);

        match syscall {
            syscall::Syscall::ConsoleWrite {
                fd: _,
                buf_ptr,
                len,
            } => {
                for i in 0..len {
                    let byte = memory
                        .read_byte(ferrous_vm::PhysAddr::new(buf_ptr.val() + i as u32))
                        .map_err(|e| TrapError::HandlerPanic(format!("Memory error: {:?}", e)))?;

                    // Driver: Write to UART
                    memory
                        .write_word(
                            ferrous_vm::PhysAddr::new(UART_BASE + UART_THR_OFFSET),
                            byte as u32,
                        )
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("UART write error: {:?}", e))
                        })?;
                }

                syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::Exit { code } => {
                info!("Thread/Process Exit: {}", code);
                self.thread_manager.exit_current_thread(code);
                self.thread_manager.yield_thread(cpu);

                if self.thread_manager.current_thread.is_none() {
                    return Err(TrapError::Halt);
                }
                Ok(VirtAddr::new(cpu.pc))
            }
            syscall::Syscall::ThreadCreate {
                entry_point,
                stack_top,
            } => {
                let result = self
                    .thread_manager
                    .create_thread(entry_point, stack_top)
                    .map(|h| syscall::SyscallReturn::Handle(h.val()))
                    .map_err(|e| TrapError::HandlerPanic(e)); // Should be SyscallError

                match result {
                    Ok(val) => syscall::Syscall::encode_result(Ok(val), cpu),
                    Err(e) => return Err(e),
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::ThreadYield => {
                syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                cpu.pc += 4;
                self.thread_manager.yield_thread(cpu);
                Ok(VirtAddr::new(cpu.pc))
            }
            syscall::Syscall::MutexCreate => {
                let id = self.next_mutex_id;
                self.next_mutex_id += 1;
                let mutex = Mutex::new(id);
                self.mutexes.insert(id, mutex);
                syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Handle(id)), cpu);
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::MutexAcquire { id } => {
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "MutexAcquire called without current thread".into(),
                        ))?;

                if let Some(mutex) = self.mutexes.get_mut(&id) {
                    if mutex.owner.is_none() {
                        mutex.owner = Some(current_handle);
                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                        Ok(VirtAddr::new(cpu.pc + 4))
                    } else {
                        mutex.wait_queue.push_back(current_handle);
                        self.thread_manager.block_current_thread();
                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                        cpu.pc += 4;
                        self.thread_manager.yield_thread(cpu);
                        Ok(VirtAddr::new(cpu.pc))
                    }
                } else {
                    syscall::Syscall::encode_result(
                        Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                        cpu,
                    );
                    Ok(VirtAddr::new(cpu.pc + 4))
                }
            }
            syscall::Syscall::MutexRelease { id } => {
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "MutexRelease called without current thread".into(),
                        ))?;

                if let Some(mutex) = self.mutexes.get_mut(&id) {
                    if mutex.owner == Some(current_handle) {
                        mutex.owner = None;
                        if let Some(next_owner) = mutex.wait_queue.pop_front() {
                            mutex.owner = Some(next_owner);
                            self.thread_manager.wake_thread(next_owner);
                        }
                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                    } else {
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        );
                    }
                } else {
                    syscall::Syscall::encode_result(
                        Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                        cpu,
                    );
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
        }
    }
}

impl TrapHandler for Kernel {
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError> {
        // Ensure current thread is tracked (lazy init of main thread)
        self.thread_manager.ensure_current_thread(cpu);

        match cause {
            TrapCause::EnvironmentCallFromU | TrapCause::EnvironmentCallFromS => {
                self.handle_syscall(cpu, memory)
            }
            TrapCause::TimerInterrupt => {
                // Preemption: Yield current thread
                self.thread_manager.yield_thread(cpu);
                Ok(VirtAddr::new(cpu.pc))
            }
            _ => Err(TrapError::Unhandled(cause)),
        }
    }
}
