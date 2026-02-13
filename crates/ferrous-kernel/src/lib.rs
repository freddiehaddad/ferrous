pub mod error;
pub mod fs;
pub mod memory;
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

    pub fn init_memory(&self, memory: &mut dyn Memory) -> Result<u32, KernelError> {
        memory::setup_kernel_address_space(memory).map_err(|e| KernelError::InitializationError(e))
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
            syscall::Syscall::Sbrk { increment } => {
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "Sbrk called without current thread".into(),
                        ))?;

                // Get current program break
                let mut current_break = 0;
                let mut root_ppn = 0;

                if let Some(tcb) = self.thread_manager.threads.get(&current_handle) {
                    current_break = tcb.program_break;
                    root_ppn = tcb.context.satp & 0x003F_FFFF; // Extract PPN from SATP
                }

                if increment == 0 {
                    syscall::Syscall::encode_result(
                        Ok(syscall::SyscallReturn::Value(current_break as i64)),
                        cpu,
                    );
                    return Ok(VirtAddr::new(cpu.pc + 4));
                }

                let new_break = (current_break as i32 + increment) as u32;

                // Align to page boundary for mapping check
                let old_page_end =
                    (current_break + memory::PAGE_SIZE - 1) & !(memory::PAGE_SIZE - 1);
                let new_page_end = (new_break + memory::PAGE_SIZE - 1) & !(memory::PAGE_SIZE - 1);

                if increment > 0 {
                    // Growing
                    if new_page_end > old_page_end {
                        // Need to allocate new pages
                        let start_page = old_page_end;
                        let end_page = new_page_end;
                        let mut page_addr = start_page;

                        while page_addr < end_page {
                            // Alloc frame
                            let frame = memory::alloc_frame();
                            // Map
                            memory::map_page(
                                memory,
                                root_ppn,
                                page_addr,
                                frame,
                                memory::PTE_R | memory::PTE_W | memory::PTE_U, // User RW
                            )
                            .map_err(|e| TrapError::HandlerPanic(e))?;

                            page_addr += memory::PAGE_SIZE;
                        }
                    }
                } else {
                    // Shrinking (Not implemented yet for safety/simplicity, just update break)
                }

                // Update TCB
                if let Some(tcb) = self.thread_manager.threads.get_mut(&current_handle) {
                    tcb.program_break = new_break;
                }

                syscall::Syscall::encode_result(
                    Ok(syscall::SyscallReturn::Value(current_break as i64)),
                    cpu,
                );
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::BlockRead { sector, buf_ptr } => {
                // Temporary implementation: Read into stack buffer, then copy to user
                let mut buffer = [0u8; 512];

                // Read from device
                match crate::fs::block::read_sector(memory, sector, &mut buffer) {
                    Ok(_) => {
                        // Copy to user memory
                        // Note: This is slow byte-by-byte copy and assumes contiguous physical memory?
                        // No, we must translate EACH page.
                        // For now, let's assume the buffer doesn't cross a page boundary or handle it simply.

                        // We need to translate buf_ptr.
                        // Since we don't have a convenient V-to-P helper exposed here yet,
                        // and `memory` is purely physical...

                        // Let's get the current root table from the thread
                        let current_handle = self
                            .thread_manager
                            .current_thread
                            .ok_or(TrapError::HandlerPanic("No current thread".into()))?;

                        let satp = self
                            .thread_manager
                            .threads
                            .get(&current_handle)
                            .unwrap()
                            .context
                            .satp;
                        let root_ppn = satp & 0x003F_FFFF;

                        for i in 0..512 {
                            let vaddr = buf_ptr.val() + i as u32;
                            // Software walk...
                            // This is getting complicated to duplicate here.
                            // Ideally we need `memory.write_virtual(satp, vaddr, byte)`

                            // Re-implement simplified walk:
                            let vpn1 = (vaddr >> 22) & 0x3FF;
                            let vpn0 = (vaddr >> 12) & 0x3FF;
                            let offset = vaddr & 0xFFF;

                            // L1
                            let l1_pte_addr =
                                ferrous_vm::PhysAddr::new((root_ppn << 12) + (vpn1 * 4));
                            let l1_pte = memory.read_word(l1_pte_addr).map_err(|e| {
                                TrapError::HandlerPanic(format!("L1 read error: {:?}", e))
                            })?;

                            if (l1_pte & memory::PTE_V) == 0 {
                                return Err(TrapError::HandlerPanic(
                                    "Page fault during syscall copy".into(),
                                ));
                            }

                            let l0_ppn = (l1_pte >> 10) & 0x3F_FFFF;

                            // L0
                            let l0_pte_addr =
                                ferrous_vm::PhysAddr::new((l0_ppn << 12) + (vpn0 * 4));
                            let l0_pte = memory.read_word(l0_pte_addr).map_err(|e| {
                                TrapError::HandlerPanic(format!("L0 read error: {:?}", e))
                            })?;

                            if (l0_pte & memory::PTE_V) == 0 {
                                return Err(TrapError::HandlerPanic(
                                    "Page fault during syscall copy".into(),
                                ));
                            }

                            let ppn = (l0_pte >> 10) & 0x3F_FFFF;
                            let paddr = (ppn << 12) | offset;

                            memory
                                .write_byte(ferrous_vm::PhysAddr::new(paddr), buffer[i as usize])
                                .map_err(|e| {
                                    TrapError::HandlerPanic(format!("User write error: {:?}", e))
                                })?;
                        }

                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                    }
                    Err(e) => {
                        // Should return error code
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        ); // Generic fail
                    }
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
