pub mod error;
pub mod fs;
pub mod memory;
pub mod sync;
pub mod syscall;
pub mod thread;
pub mod types;

use crate::error::KernelError;
use crate::sync::Mutex;
use crate::thread::tcb::FileDescriptor;
use ferrous_vm::{Cpu, Memory, PhysAddr, TrapCause, TrapError, TrapHandler, VirtAddr};
use goblin::elf;
use log::{debug, info, warn};
use std::collections::HashMap;
use thread::ThreadManager;

pub struct Kernel {
    thread_manager: ThreadManager,
    mutexes: HashMap<u32, Mutex>,
    next_mutex_id: u32,
    file_system: Option<fs::FileSystem>,
}

const UART_BASE: u32 = 0x1000_0000;
const UART_THR_OFFSET: u32 = 0x00;
const UART_RBR_OFFSET: u32 = 0x00;
const UART_LSR_OFFSET: u32 = 0x05;

impl Kernel {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {
            thread_manager: ThreadManager::new(),
            mutexes: HashMap::new(),
            next_mutex_id: 1,
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
        let elf = elf::Elf::parse(elf_data)
            .map_err(|e| TrapError::HandlerPanic(format!("Bootstrap: Invalid ELF: {:?}", e)))?;

        // 1. Create Address Space
        let satp_val =
            memory::create_user_address_space(memory).map_err(TrapError::HandlerPanic)?;
        let root_ppn = satp_val & 0x003F_FFFF;

        // 2. Load Segments
        let mut max_vaddr = 0;
        for ph in elf.program_headers.iter() {
            if ph.p_type == elf::program_header::PT_LOAD {
                let file_start = ph.p_offset as usize;
                let file_len = ph.p_filesz as usize;
                let segment_data = &elf_data[file_start..(file_start + file_len)];

                let vaddr_start = ph.p_vaddr as u32;
                let mem_len = ph.p_memsz as u32;

                let mut current_vaddr = vaddr_start;
                let end_vaddr = vaddr_start + mem_len;

                if end_vaddr > max_vaddr {
                    max_vaddr = end_vaddr;
                }

                while current_vaddr < end_vaddr {
                    let page_base = current_vaddr & !(memory::PAGE_SIZE - 1);
                    // Check if already mapped (segment overlap?) or alloc new
                    let paddr_base = match translate_vaddr(memory, satp_val, page_base) {
                        Ok(p) => p & !(memory::PAGE_SIZE - 1),
                        Err(_) => {
                            let frame = memory::alloc_frame();
                            let flags =
                                memory::PTE_R | memory::PTE_W | memory::PTE_U | memory::PTE_X;
                            memory::map_page(memory, root_ppn, page_base, frame, flags)
                                .map_err(|e| TrapError::HandlerPanic(e))?;
                            // Zero fill
                            for i in 0..memory::PAGE_SIZE {
                                memory.write_byte(PhysAddr::new(frame + i), 0).unwrap();
                            }
                            frame
                        }
                    };

                    let page_offset = current_vaddr & (memory::PAGE_SIZE - 1);
                    let bytes_available_in_page = memory::PAGE_SIZE - page_offset;
                    let bytes_to_end = end_vaddr - current_vaddr;
                    let chunk_size = bytes_available_in_page.min(bytes_to_end);

                    let segment_offset = (current_vaddr - vaddr_start) as usize;

                    if segment_offset < file_len {
                        let data_remaining = file_len - segment_offset;
                        let copy_size = (chunk_size as usize).min(data_remaining);

                        for i in 0..copy_size {
                            let b = segment_data[segment_offset + i];
                            memory
                                .write_byte(PhysAddr::new(paddr_base + page_offset + i as u32), b)
                                .map_err(|e| {
                                    TrapError::HandlerPanic(format!(
                                        "Bootstrap write error: {:?}",
                                        e
                                    ))
                                })?;
                        }
                    }
                    current_vaddr += chunk_size;
                }
            }
        }

        // 3. Setup Stack
        let stack_top = 0xF000_0000u32;
        let stack_pages = 4;
        for i in 0..stack_pages {
            let vaddr = stack_top - ((i + 1) * memory::PAGE_SIZE);
            let frame = memory::alloc_frame();
            memory::map_page(
                memory,
                root_ppn,
                vaddr,
                frame,
                memory::PTE_R | memory::PTE_W | memory::PTE_U,
            )
            .map_err(TrapError::HandlerPanic)?;
        }

        // 4. Push Arguments
        let mut current_sp = stack_top;

        // 4a. Push String Data
        let mut arg_vaddrs = Vec::with_capacity(args.len());
        for arg in args {
            let arg_bytes = arg.as_bytes();
            current_sp -= arg_bytes.len() as u32; // No null terminator needed for slice access, but standard is null-term?
                                                  // Shell expects &str parts, but Exec passes bytes.
                                                  // Let's stick to simple copy.
            let dest = VirtAddr::new(current_sp);
            copy_to_user(memory, satp_val, arg_bytes, dest)?;
            arg_vaddrs.push(current_sp);
        }

        // 4b. Push Argv Array (ptr, len) for Rust-style args
        let argv_size = (args.len() * 8) as u32;
        current_sp -= argv_size;
        current_sp &= !3;
        let argv_base = current_sp;

        for (i, vaddr) in arg_vaddrs.iter().enumerate() {
            let len = args[i].len() as u32;
            let desc_addr = argv_base + (i * 8) as u32;

            // Write ptr
            let paddr_ptr = translate_vaddr(memory, satp_val, desc_addr)?;
            memory
                .write_word(PhysAddr::new(paddr_ptr), *vaddr)
                .map_err(|e| {
                    TrapError::HandlerPanic(format!("Bootstrap arg ptr write error: {:?}", e))
                })?;

            // Write len
            let paddr_len = translate_vaddr(memory, satp_val, desc_addr + 4)?;
            memory
                .write_word(PhysAddr::new(paddr_len), len)
                .map_err(|e| {
                    TrapError::HandlerPanic(format!("Bootstrap arg len write error: {:?}", e))
                })?;
        }

        // Align Stack
        current_sp &= !15;

        // 5. Create Thread
        let entry_point = VirtAddr::new(elf.entry as u32);
        let handle = self
            .thread_manager
            .create_thread(entry_point, current_sp)
            .map_err(TrapError::HandlerPanic)?;

        if let Some(tcb) = self.thread_manager.threads.get_mut(&handle) {
            tcb.context.satp = satp_val;
            tcb.context.mode = ferrous_vm::PrivilegeMode::User; // Ensure User Mode

            // Set a0 (argc), a1 (argv)
            tcb.context
                .write_reg(ferrous_vm::Register::new(10).unwrap(), args.len() as u32);
            tcb.context
                .write_reg(ferrous_vm::Register::new(11).unwrap(), argv_base);

            // Set program break to end of loaded segments
            // Align to next page boundary for cleanliness, though not strictly required
            let heap_start = (max_vaddr + memory::PAGE_SIZE - 1) & !(memory::PAGE_SIZE - 1);
            tcb.program_break = heap_start;
            info!(
                "Bootstrap: Heap starts at {:#x} (Segment end: {:#x})",
                heap_start, max_vaddr
            );
        }

        self.thread_manager.current_thread = Some(handle);

        Ok((
            entry_point,
            satp_val,
            current_sp,
            args.len() as u32,
            argv_base,
        ))
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

        // Get current thread context for SATP (needed for copy_from_user)
        let satp = if let Some(current) = self.thread_manager.current_thread {
            if let Some(tcb) = self.thread_manager.threads.get(&current) {
                tcb.context.satp
            } else {
                0
            }
        } else {
            0
        };

        match syscall {
            syscall::Syscall::ConsoleWrite {
                fd: _,
                buf_ptr,
                len,
            } => {
                let mut buf = vec![0u8; len];
                copy_from_user(memory, satp, buf_ptr, &mut buf)?;

                for byte in buf {
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
            syscall::Syscall::ConsoleRead {
                fd: _,
                buf_ptr,
                len,
            } => {
                if len == 0 {
                    syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Value(0)), cpu);
                    return Ok(VirtAddr::new(cpu.pc + 4));
                }

                let mut read_buf = Vec::new();

                // 1. Blocking read for the first byte
                // Accessing memory at UART_BASE triggers the device read
                let val = memory
                    .read_word(ferrous_vm::PhysAddr::new(UART_BASE + UART_RBR_OFFSET))
                    .map_err(|e| TrapError::HandlerPanic(format!("UART read error: {:?}", e)))?;

                if val == 0 {
                    // EOF on first byte
                    syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Value(0)), cpu);
                    return Ok(VirtAddr::new(cpu.pc + 4));
                }
                read_buf.push(val as u8);

                // 2. Non-blocking read for subsequent bytes
                let limit = len.min(1024);
                while read_buf.len() < limit {
                    let lsr = memory
                        .read_word(ferrous_vm::PhysAddr::new(UART_BASE + UART_LSR_OFFSET))
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("UART LSR read error: {:?}", e))
                        })?;

                    if (lsr & 0x01) == 0 {
                        break; // No more data
                    }

                    let val = memory
                        .read_word(ferrous_vm::PhysAddr::new(UART_BASE + UART_RBR_OFFSET))
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("UART read error: {:?}", e))
                        })?;

                    if val == 0 {
                        break; // EOF
                    }
                    read_buf.push(val as u8);

                    if val == 10 || val == 13 {
                        break; // Newline
                    }
                }

                // 3. Copy to user
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

                copy_to_user(memory, satp, &read_buf, buf_ptr)?;

                syscall::Syscall::encode_result(
                    Ok(syscall::SyscallReturn::Value(read_buf.len() as i64)),
                    cpu,
                );
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
                debug!(
                    "ThreadCreate: entry={:?}, stack={:#x}",
                    entry_point, stack_top
                );
                let result = self
                    .thread_manager
                    .create_thread(entry_point, stack_top)
                    .map(|h| syscall::SyscallReturn::Handle(h.val()))
                    .map_err(TrapError::HandlerPanic); // Should be SyscallError

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

                        debug!("Sbrk: Allocating {} bytes. Old break: {:#x}. Mapping pages from {:#x} to {:#x}", increment, current_break, start_page, end_page);

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
                            .map_err(TrapError::HandlerPanic)?;

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
                let mut buffer = [0u8; 512];
                match crate::fs::block::read_sector(memory, sector, &mut buffer) {
                    Ok(_) => {
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

                        copy_to_user(memory, satp, &buffer, buf_ptr)?;
                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                    }
                    Err(_) => {
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        );
                    }
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::FileOpen { path_ptr, path_len } => {
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "FileOpen: No current thread".into(),
                        ))?;

                let satp = self
                    .thread_manager
                    .threads
                    .get(&current_handle)
                    .unwrap()
                    .context
                    .satp;

                let mut path_bytes = vec![0u8; path_len];
                copy_from_user(memory, satp, path_ptr, &mut path_bytes)?;

                let path_str = String::from_utf8(path_bytes)
                    .map_err(|_| TrapError::HandlerPanic("Invalid UTF-8 path".into()))?;

                let inode_id = if let Some(fs) = &self.file_system {
                    fs.find_inode(memory, &path_str)
                        .map_err(|_| crate::error::SyscallError::InvalidSyscallNumber(0))
                } else {
                    Err(crate::error::SyscallError::InvalidSyscallNumber(0))
                };

                match inode_id {
                    Ok(id) => {
                        let tcb = self
                            .thread_manager
                            .threads
                            .get_mut(&current_handle)
                            .unwrap();
                        // Find free FD
                        let fd_idx = tcb.file_descriptors.len();
                        tcb.file_descriptors.push(Some(FileDescriptor {
                            inode_id: id,
                            offset: 0,
                            flags: 0,
                        }));
                        syscall::Syscall::encode_result(
                            Ok(syscall::SyscallReturn::Handle(fd_idx as u32)),
                            cpu,
                        );
                    }
                    Err(_) => {
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        );
                    }
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::WaitPid { pid } => {
                let target = crate::types::ThreadHandle::new(pid)
                    .ok_or(TrapError::HandlerPanic("Invalid pid 0".into()))?;

                match self.thread_manager.wait_current_thread(target) {
                    Ok(Some(exit_code)) => {
                        // Already terminated
                        syscall::Syscall::encode_result(
                            Ok(syscall::SyscallReturn::Value(exit_code as i64)),
                            cpu,
                        );
                        Ok(VirtAddr::new(cpu.pc + 4))
                    }
                    Ok(None) => {
                        // Blocked. Return placeholder (will be overwritten by waker)
                        syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                        cpu.pc += 4;
                        self.thread_manager.yield_thread(cpu);
                        Ok(VirtAddr::new(cpu.pc))
                    }
                    Err(e) => {
                        warn!("WaitPid failed: {}", e);
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        );
                        Ok(VirtAddr::new(cpu.pc + 4))
                    }
                }
            }
            syscall::Syscall::FileRead { fd, buf_ptr, len } => {
                debug!("FileRead: fd={}, buf={:?}, len={}", fd, buf_ptr, len);
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "FileRead: No current thread".into(),
                        ))?;

                let (inode_id, offset, satp) = {
                    let tcb = self.thread_manager.threads.get(&current_handle).unwrap();
                    if (fd as usize) < tcb.file_descriptors.len() {
                        if let Some(desc) = &tcb.file_descriptors[fd as usize] {
                            (desc.inode_id, desc.offset, tcb.context.satp)
                        } else {
                            syscall::Syscall::encode_result(
                                Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                                cpu,
                            );
                            return Ok(VirtAddr::new(cpu.pc + 4));
                        }
                    } else {
                        syscall::Syscall::encode_result(
                            Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                            cpu,
                        );
                        return Ok(VirtAddr::new(cpu.pc + 4));
                    }
                };

                if let Some(fs) = &self.file_system {
                    debug!("FileRead: Reading inode {}", inode_id);
                    let inode = fs
                        .read_inode(memory, inode_id)
                        .map_err(|e| TrapError::HandlerPanic(format!("Read Inode: {:?}", e)))?;
                    debug!("FileRead: Inode size: {}", inode.size);

                    let mut total_read = 0;
                    let mut temp_buf = [0u8; 512];
                    let mut current_offset = offset;
                    let mut remaining = len;

                    while remaining > 0 {
                        let chunk_size = remaining.min(512);
                        debug!(
                            "FileRead: Reading chunk size {} at offset {}",
                            chunk_size, current_offset
                        );
                        let bytes = fs
                            .read_data(memory, &inode, current_offset, &mut temp_buf[..chunk_size])
                            .map_err(|e| TrapError::HandlerPanic(format!("Read Data: {:?}", e)))?;

                        debug!("FileRead: Read {} bytes", bytes);
                        if bytes == 0 {
                            break;
                        }

                        debug!("FileRead: Copying to user at {:?}", buf_ptr);
                        copy_to_user(
                            memory,
                            satp,
                            &temp_buf[..bytes],
                            VirtAddr::new(buf_ptr.val() + total_read as u32),
                        )?;

                        total_read += bytes;
                        current_offset += bytes as u32;
                        remaining -= bytes;
                    }

                    debug!("FileRead: Updating offset to {}", current_offset);
                    let tcb = self
                        .thread_manager
                        .threads
                        .get_mut(&current_handle)
                        .unwrap();
                    if let Some(desc) = tcb.file_descriptors[fd as usize].as_mut() {
                        desc.offset = current_offset;
                    }

                    syscall::Syscall::encode_result(
                        Ok(syscall::SyscallReturn::Value(total_read as i64)),
                        cpu,
                    );
                } else {
                    syscall::Syscall::encode_result(
                        Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                        cpu,
                    );
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }

            syscall::Syscall::FileClose { fd } => {
                let current_handle =
                    self.thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic(
                            "FileClose: No current thread".into(),
                        ))?;

                let tcb = self
                    .thread_manager
                    .threads
                    .get_mut(&current_handle)
                    .unwrap();
                if (fd as usize) < tcb.file_descriptors.len() {
                    tcb.file_descriptors[fd as usize] = None;
                    syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                } else {
                    syscall::Syscall::encode_result(
                        Err(crate::error::SyscallError::InvalidSyscallNumber(0)),
                        cpu,
                    );
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
            syscall::Syscall::Exec {
                path_ptr,
                path_len,
                args_ptr,
                args_len,
            } => {
                info!("Exec syscall");
                let current_handle = self
                    .thread_manager
                    .current_thread
                    .ok_or(TrapError::HandlerPanic("Exec: No current thread".into()))?;

                let satp = self
                    .thread_manager
                    .threads
                    .get(&current_handle)
                    .unwrap()
                    .context
                    .satp;

                // 1. Read Path
                let mut path_bytes = vec![0u8; path_len];
                copy_from_user(memory, satp, path_ptr, &mut path_bytes)?;
                let path_str = String::from_utf8(path_bytes)
                    .map_err(|_| TrapError::HandlerPanic("Invalid UTF-8 path".into()))?;

                info!("Exec loading: {}", path_str);

                // 1.5 Read Arguments
                let mut args_vec: Vec<Vec<u8>> = Vec::with_capacity(args_len);
                if args_len > 0 {
                    let mut arg_descriptors_bytes = vec![0u8; args_len * 8];
                    copy_from_user(memory, satp, args_ptr, &mut arg_descriptors_bytes)?;

                    for i in 0..args_len {
                        let offset = i * 8;
                        let ptr = u32::from_le_bytes(
                            arg_descriptors_bytes[offset..offset + 4]
                                .try_into()
                                .unwrap(),
                        );
                        let len = u32::from_le_bytes(
                            arg_descriptors_bytes[offset + 4..offset + 8]
                                .try_into()
                                .unwrap(),
                        );

                        let mut arg_data = vec![0u8; len as usize];
                        copy_from_user(memory, satp, VirtAddr::new(ptr), &mut arg_data)?;
                        args_vec.push(arg_data);
                    }
                }

                // 2. Read File from FS
                let file_data = if let Some(fs) = &self.file_system {
                    let inode_id = fs
                        .find_inode(memory, &path_str)
                        .map_err(|_| TrapError::HandlerPanic("Exec: File not found".into()))?;
                    let inode = fs.read_inode(memory, inode_id).map_err(|e| {
                        TrapError::HandlerPanic(format!("Exec: Read Inode: {:?}", e))
                    })?;

                    let mut data = vec![0u8; inode.size as usize];
                    let mut offset = 0;
                    while offset < inode.size {
                        let remaining = inode.size - offset;
                        let chunk_size = remaining.min(512);
                        let bytes_read = fs
                            .read_data(
                                memory,
                                &inode,
                                offset,
                                &mut data[offset as usize..(offset + chunk_size) as usize],
                            )
                            .map_err(|e| {
                                TrapError::HandlerPanic(format!("Exec: Read Data: {:?}", e))
                            })?;
                        if bytes_read == 0 {
                            break;
                        }
                        offset += bytes_read as u32;
                    }
                    data
                } else {
                    return Err(TrapError::HandlerPanic("Exec: No filesystem".into()));
                };

                // 3. Parse ELF
                let elf = elf::Elf::parse(&file_data)
                    .map_err(|e| TrapError::HandlerPanic(format!("Exec: Invalid ELF: {:?}", e)))?;

                // 4. Create Address Space
                let satp_val =
                    memory::create_user_address_space(memory).map_err(TrapError::HandlerPanic)?;
                let root_ppn = satp_val & 0x003F_FFFF;

                // 5. Load Segments
                let mut max_vaddr = 0;
                for ph in elf.program_headers.iter() {
                    if ph.p_type == elf::program_header::PT_LOAD {
                        let file_start = ph.p_offset as usize;
                        let file_len = ph.p_filesz as usize;
                        let segment_data = &file_data[file_start..(file_start + file_len)];

                        let vaddr_start = ph.p_vaddr as u32;
                        let mem_len = ph.p_memsz as u32;

                        let mut current_vaddr = vaddr_start;
                        let end_vaddr = vaddr_start + mem_len;

                        if end_vaddr > max_vaddr {
                            max_vaddr = end_vaddr;
                        }

                        while current_vaddr < end_vaddr {
                            let page_base = current_vaddr & !(memory::PAGE_SIZE - 1);
                            let paddr_base = match translate_vaddr(memory, satp_val, page_base) {
                                Ok(p) => p & !(memory::PAGE_SIZE - 1),
                                Err(_) => {
                                    let frame = memory::alloc_frame();
                                    let flags = memory::PTE_R
                                        | memory::PTE_W
                                        | memory::PTE_U
                                        | memory::PTE_X;
                                    memory::map_page(memory, root_ppn, page_base, frame, flags)
                                        .map_err(TrapError::HandlerPanic)?;
                                    for i in 0..memory::PAGE_SIZE {
                                        memory.write_byte(PhysAddr::new(frame + i), 0).unwrap();
                                    }
                                    frame
                                }
                            };

                            let page_offset = current_vaddr & (memory::PAGE_SIZE - 1);
                            let bytes_available_in_page = memory::PAGE_SIZE - page_offset;
                            let bytes_to_end = end_vaddr - current_vaddr;
                            let chunk_size = bytes_available_in_page.min(bytes_to_end);

                            let segment_offset = (current_vaddr - vaddr_start) as usize;

                            if segment_offset < file_len {
                                let data_remaining = file_len - segment_offset;
                                let copy_size = (chunk_size as usize).min(data_remaining);

                                for i in 0..copy_size {
                                    let b = segment_data[segment_offset + i];
                                    memory
                                        .write_byte(
                                            PhysAddr::new(paddr_base + page_offset + i as u32),
                                            b,
                                        )
                                        .map_err(|e| {
                                            TrapError::HandlerPanic(format!("Write error: {:?}", e))
                                        })?;
                                }
                            }
                            current_vaddr += chunk_size;
                        }
                    }
                }

                // 6. Setup Stack
                let stack_top = 0xF000_0000u32;
                let stack_pages = 4;
                for i in 0..stack_pages {
                    let vaddr = stack_top - ((i + 1) * memory::PAGE_SIZE);
                    let frame = memory::alloc_frame();
                    memory::map_page(
                        memory,
                        root_ppn,
                        vaddr,
                        frame,
                        memory::PTE_R | memory::PTE_W | memory::PTE_U,
                    )
                    .map_err(TrapError::HandlerPanic)?;
                }

                // 7. Push Arguments to Stack
                let mut current_sp = stack_top;

                // 7a. Push String Data
                let mut arg_vaddrs = Vec::with_capacity(args_len);
                for arg_data in &args_vec {
                    current_sp -= arg_data.len() as u32;
                    let dest = VirtAddr::new(current_sp);
                    copy_to_user(memory, satp_val, arg_data, dest)?;
                    arg_vaddrs.push(current_sp);
                }

                // 7b. Push Argv Array (Descriptors: ptr, len)
                // We need to push args_len * 8 bytes
                let argv_size = (args_len * 8) as u32;
                current_sp -= argv_size;
                current_sp &= !3; // Align to 4 bytes
                let argv_base = current_sp;

                for (i, vaddr) in arg_vaddrs.iter().enumerate() {
                    let len = args_vec[i].len() as u32;
                    let desc_addr = argv_base + (i * 8) as u32;

                    // Write ptr
                    let paddr_ptr = translate_vaddr(memory, satp_val, desc_addr)?;
                    memory
                        .write_word(PhysAddr::new(paddr_ptr), *vaddr)
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("Stack write error: {:?}", e))
                        })?;

                    // Write len
                    let paddr_len = translate_vaddr(memory, satp_val, desc_addr + 4)?;
                    memory
                        .write_word(PhysAddr::new(paddr_len), len)
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("Stack write error: {:?}", e))
                        })?;
                }

                // 7c. Align Stack to 16 bytes
                current_sp &= !15;

                // 8. Create Thread/Process
                let entry_point = VirtAddr::new(elf.entry as u32);
                let handle = self
                    .thread_manager
                    .create_thread(entry_point, current_sp)
                    .map_err(TrapError::HandlerPanic)?;

                if let Some(tcb) = self.thread_manager.threads.get_mut(&handle) {
                    tcb.context.satp = satp_val;
                    // Set argc (a0) and argv (a1)
                    tcb.context
                        .write_reg(ferrous_vm::Register::new(10).unwrap(), args_len as u32);
                    tcb.context
                        .write_reg(ferrous_vm::Register::new(11).unwrap(), argv_base);

                    // Set program break
                    let heap_start = (max_vaddr + memory::PAGE_SIZE - 1) & !(memory::PAGE_SIZE - 1);
                    tcb.program_break = heap_start;
                    info!(
                        "Exec: Loaded max_vaddr={:#x}, Heap starts at {:#x}",
                        max_vaddr, heap_start
                    );
                }

                info!("Exec spawned new process with handle: {:?}", handle);
                syscall::Syscall::encode_result(
                    Ok(syscall::SyscallReturn::Handle(handle.val())),
                    cpu,
                );
                Ok(VirtAddr::new(cpu.pc + 4))
            }
        }
    }
}

impl TrapHandler for Kernel {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
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

// Helper functions for user memory access
fn translate_vaddr(memory: &mut dyn Memory, satp: u32, vaddr: u32) -> Result<u32, TrapError> {
    // Check Mode (MSB of SATP)
    // If Mode is 0, Bare mode (Physical = Virtual)
    if (satp & 0x8000_0000) == 0 {
        return Ok(vaddr);
    }

    let root_ppn = satp & 0x003F_FFFF;
    let vpn1 = (vaddr >> 22) & 0x3FF;
    let vpn0 = (vaddr >> 12) & 0x3FF;
    let offset = vaddr & 0xFFF;

    let l1_pte_addr = ferrous_vm::PhysAddr::new((root_ppn << 12) + (vpn1 * 4));
    let l1_pte = memory
        .read_word(l1_pte_addr)
        .map_err(|e| TrapError::HandlerPanic(format!("L1 read error: {:?}", e)))?;

    if (l1_pte & crate::memory::PTE_V) == 0 {
        return Err(TrapError::HandlerPanic("Page fault (L1 invalid)".into()));
    }

    let l0_ppn = (l1_pte >> 10) & 0x3F_FFFF;
    let l0_pte_addr = ferrous_vm::PhysAddr::new((l0_ppn << 12) + (vpn0 * 4));
    let l0_pte = memory
        .read_word(l0_pte_addr)
        .map_err(|e| TrapError::HandlerPanic(format!("L0 read error: {:?}", e)))?;

    if (l0_pte & crate::memory::PTE_V) == 0 {
        return Err(TrapError::HandlerPanic("Page fault (L0 invalid)".into()));
    }

    let ppn = (l0_pte >> 10) & 0x3F_FFFF;
    let paddr = (ppn << 12) | offset;
    Ok(paddr)
}

fn copy_from_user(
    memory: &mut dyn Memory,
    satp: u32,
    src_ptr: VirtAddr,
    dest: &mut [u8],
) -> Result<(), TrapError> {
    for (i, byte) in dest.iter_mut().enumerate() {
        let vaddr = src_ptr.val() + i as u32;
        let paddr = translate_vaddr(memory, satp, vaddr)?;
        *byte = memory
            .read_byte(ferrous_vm::PhysAddr::new(paddr))
            .map_err(|e| TrapError::HandlerPanic(format!("User read error: {:?}", e)))?;
    }
    Ok(())
}

fn copy_to_user(
    memory: &mut dyn Memory,
    satp: u32,
    src: &[u8],
    dest_ptr: VirtAddr,
) -> Result<(), TrapError> {
    for (i, byte) in src.iter().enumerate() {
        let vaddr = dest_ptr.val() + i as u32;
        let paddr = translate_vaddr(memory, satp, vaddr)?;
        memory
            .write_byte(ferrous_vm::PhysAddr::new(paddr), *byte)
            .map_err(|e| TrapError::HandlerPanic(format!("User write error: {:?}", e)))?;
    }
    Ok(())
}
