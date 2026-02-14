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
        Syscall::WaitPid { pid } => {
            let target = crate::types::ThreadHandle::new(pid)
                .ok_or(TrapError::HandlerPanic("Invalid pid 0".into()))?;

            match thread_manager.wait_current_thread(target) {
                Ok(Some(exit_code)) => {
                    // Already terminated
                    Syscall::encode_result(Ok(SyscallReturn::Value(exit_code as i64)), cpu);
                    Ok(VirtAddr::new(cpu.pc + 4))
                }
                Ok(None) => {
                    // Blocked. Return placeholder (will be overwritten by waker)
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                    cpu.pc += 4;
                    thread_manager.yield_thread(cpu);
                    Ok(VirtAddr::new(cpu.pc))
                }
                Err(e) => {
                    warn!("WaitPid failed: {}", e);
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                    Ok(VirtAddr::new(cpu.pc + 4))
                }
            }
        }
        Syscall::Exec {
            path_ptr,
            path_len,
            args_ptr,
            args_len,
        } => {
            info!("Exec syscall");
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic("Exec: No current thread".into()))?;

            let satp = thread_manager
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
            let file_data = if let Some(fs) = file_system {
                let inode_id = fs
                    .find_inode(memory, &path_str)
                    .map_err(|_| TrapError::HandlerPanic("Exec: File not found".into()))?;
                let inode = fs
                    .read_inode(memory, inode_id)
                    .map_err(|e| TrapError::HandlerPanic(format!("Exec: Read Inode: {:?}", e)))?;

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
                                let flags =
                                    memory::PTE_R | memory::PTE_W | memory::PTE_U | memory::PTE_X;
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
                    .map_err(|e| TrapError::HandlerPanic(format!("Stack write error: {:?}", e)))?;

                // Write len
                let paddr_len = translate_vaddr(memory, satp_val, desc_addr + 4)?;
                memory
                    .write_word(PhysAddr::new(paddr_len), len)
                    .map_err(|e| TrapError::HandlerPanic(format!("Stack write error: {:?}", e)))?;
            }

            // 7c. Align Stack to 16 bytes
            current_sp &= !15;

            // 8. Create Thread/Process
            let entry_point = VirtAddr::new(elf.entry as u32);
            let handle = thread_manager
                .create_thread(entry_point, current_sp)
                .map_err(TrapError::HandlerPanic)?;

            if let Some(tcb) = thread_manager.threads.get_mut(&handle) {
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
            Syscall::encode_result(Ok(SyscallReturn::Handle(handle.val())), cpu);
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        _ => Err(TrapError::HandlerPanic("Process: Unhandled syscall".into())),
    }
}
