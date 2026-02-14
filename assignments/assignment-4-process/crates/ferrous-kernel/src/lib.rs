#![no_std]

extern crate alloc;

pub mod error;
pub mod fs;
pub mod memory;
pub mod process;
pub mod sync;
pub mod syscall;
pub mod thread;
pub mod types;

use crate::error::KernelError;
use crate::fs::Pipe;

use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use ferrous_vm::{Cpu, Memory, TrapCause, TrapError, TrapHandler, VirtAddr};
use log::debug;
use thread::ThreadManager;

pub struct Kernel {
    thread_manager: ThreadManager,

    mutexes: BTreeMap<u32, Mutex>,
    next_mutex_id: u32,
    pipes: BTreeMap<u32, Pipe>,
    next_pipe_id: u32,
    file_system: Option<fs::FileSystem>,
}

impl Kernel {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {
            thread_manager: ThreadManager::new(),
            mutexes: BTreeMap::new(),
            next_mutex_id: 1,
            pipes: BTreeMap::new(),
            next_pipe_id: 1,
            file_system: None,
        })
    }

    pub fn init_memory(&mut self, memory: &mut dyn Memory) -> Result<u32, KernelError> {
        let satp =
            memory::setup_kernel_address_space(memory).map_err(KernelError::InitializationError)?;

        // Try to mount FS
        match fs::FileSystem::mount(memory) {
            Ok(fs) => {
                self.file_system = Some(fs);
            }
            Err(e) => {
                // Warning only, maybe no disk attached
                log::warn!("Failed to mount filesystem: {:?}", e);
            }
        }

        Ok(satp)
    }

    pub fn bootstrap_process(
        &mut self,
        memory: &mut dyn Memory,
        elf_data: &[u8],
        args: &[String],
    ) -> Result<(VirtAddr, u32, u32, u32, u32), TrapError> {
        process::bootstrap_process(&mut self.thread_manager, memory, elf_data, args)
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
            syscall::Syscall::Pipe { .. }
            | syscall::Syscall::FileWrite { .. }
            | syscall::Syscall::ConsoleRead { .. }
            | syscall::Syscall::BlockRead { .. }
            | syscall::Syscall::FileOpen { .. }
            | syscall::Syscall::FileRead { .. }
            | syscall::Syscall::FileClose { .. } => fs::syscalls::handle_syscall(
                syscall,
                &mut self.thread_manager,
                &mut self.file_system,
                &mut self.pipes,
                &mut self.next_pipe_id,
                memory,
                cpu,
            ),
            syscall::Syscall::Exit { .. }
            | syscall::Syscall::ThreadCreate { .. }
            | syscall::Syscall::ThreadYield => {
                thread::syscalls::handle_syscall(syscall, &mut self.thread_manager, cpu)
            }
            syscall::Syscall::MutexCreate
            | syscall::Syscall::MutexAcquire { .. }
            | syscall::Syscall::MutexRelease { .. } => sync::syscalls::handle_syscall(
                syscall,
                &mut self.thread_manager,
                &mut self.mutexes,
                &mut self.next_mutex_id,
                cpu,
            ),
            syscall::Syscall::Sbrk { increment } => {
                memory::handle_sbrk(increment, &mut self.thread_manager, memory, cpu)
            }
            syscall::Syscall::WaitPid { .. } | syscall::Syscall::Exec { .. } => {
                process::syscalls::handle_syscall(
                    syscall,
                    &mut self.thread_manager,
                    &mut self.file_system,
                    memory,
                    cpu,
                )
            }
        }
    }
}

impl TrapHandler for Kernel {
    fn as_any(&mut self) -> &mut dyn core::any::Any {
        self
    }

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
