use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use ferrous_vm::{Cpu, Memory, PhysAddr, TrapError, VirtAddr};
use log::debug;

use crate::error::SyscallError;
use crate::fs::{FileSystem, Pipe};
use crate::memory::{copy_from_user, copy_to_user};
use crate::syscall::{Syscall, SyscallReturn};
use crate::thread::tcb::FileDescriptor;
use crate::thread::ThreadManager;

const UART_BASE: u32 = 0x1000_0000;
const UART_THR_OFFSET: u32 = 0x00;
const UART_RBR_OFFSET: u32 = 0x00;
const UART_LSR_OFFSET: u32 = 0x05;

pub fn handle_syscall(
    syscall: Syscall,
    thread_manager: &mut ThreadManager,
    file_system: &mut Option<FileSystem>,
    pipes: &mut BTreeMap<u32, Pipe>,
    next_pipe_id: &mut u32,
    memory: &mut dyn Memory,
    cpu: &mut Cpu,
) -> Result<VirtAddr, TrapError> {
    match syscall {
        Syscall::Pipe { pipe_array_ptr } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic("Pipe: No current thread".into()))?;

            let pipe_id = *next_pipe_id;
            *next_pipe_id += 1;

            let pipe = Pipe {
                buffer: VecDeque::new(),
                read_open: true,
                write_open: true,
                wait_queue: VecDeque::new(),
            };
            pipes.insert(pipe_id, pipe);

            let tcb = thread_manager.threads.get_mut(&current_handle).unwrap();

            // Read FD
            let read_fd = tcb.file_descriptors.len() as u32;
            tcb.file_descriptors.push(Some(FileDescriptor::Pipe {
                pipe_id,
                is_write: false,
            }));

            // Write FD
            let write_fd = tcb.file_descriptors.len() as u32;
            tcb.file_descriptors.push(Some(FileDescriptor::Pipe {
                pipe_id,
                is_write: true,
            }));

            let satp = tcb.context.satp;
            let mut fd_array = [0u8; 8];
            fd_array[0..4].copy_from_slice(&read_fd.to_le_bytes());
            fd_array[4..8].copy_from_slice(&write_fd.to_le_bytes());
            copy_to_user(memory, satp, &fd_array, pipe_array_ptr)?;

            Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::FileWrite { fd, buf_ptr, len } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "FileWrite: No current thread".into(),
                ))?;

            let tcb = thread_manager.threads.get(&current_handle).unwrap();
            let satp = tcb.context.satp;
            let descriptor = if (fd as usize) < tcb.file_descriptors.len() {
                tcb.file_descriptors[fd as usize]
            } else {
                None
            };

            let mut buf = vec![0u8; len];
            copy_from_user(memory, satp, buf_ptr, &mut buf)?;

            let mut result = Ok(SyscallReturn::Value(len as i64));
            let mut to_wake = Vec::new();

            // Check for UART (Stdout/Stderr)
            // If fd is 1 or 2 and not remapped, output to UART
            if (fd == 1 || fd == 2) && descriptor.is_none() {
                // No locking implemented yet, direct write is thread-safe enough for now
                // as MMIO is atomic per word write? No, characters might interleave.
                // But it shouldn't hang.
                for byte in buf {
                    memory
                        .write_word(PhysAddr::new(UART_BASE + UART_THR_OFFSET), byte as u32)
                        .map_err(|e| {
                            TrapError::HandlerPanic(format!("UART write error: {:?}", e))
                        })?;
                }
            } else if let Some(FileDescriptor::Pipe { pipe_id, is_write }) = descriptor {
                if !is_write {
                    result = Err(SyscallError::InvalidSyscallNumber(0));
                } else if let Some(pipe) = pipes.get_mut(&pipe_id) {
                    if !pipe.read_open {
                        // Broken pipe
                        result = Err(SyscallError::InvalidSyscallNumber(0));
                    } else {
                        pipe.buffer.extend(buf.iter());
                        while let Some(h) = pipe.wait_queue.pop_front() {
                            to_wake.push(h);
                        }
                    }
                } else {
                    result = Err(SyscallError::InvalidSyscallNumber(0));
                }
            } else {
                // Unknown FD or File (not supported)
                result = Err(SyscallError::InvalidSyscallNumber(0));
            }

            for h in to_wake {
                thread_manager.wake_thread(h);
            }

            Syscall::encode_result(result, cpu);
            Ok(VirtAddr::new(cpu.pc + 4))
        }

        Syscall::ConsoleRead {
            fd: _,
            buf_ptr,
            len,
        } => {
            if len == 0 {
                Syscall::encode_result(Ok(SyscallReturn::Value(0)), cpu);
                return Ok(VirtAddr::new(cpu.pc + 4));
            }

            let mut read_buf = Vec::new();

            // 1. Blocking read for the first byte
            // Accessing memory at UART_BASE triggers the device read
            let val = memory
                .read_word(PhysAddr::new(UART_BASE + UART_RBR_OFFSET))
                .map_err(|e| TrapError::HandlerPanic(format!("UART read error: {:?}", e)))?;

            if val == 0 {
                // EOF on first byte
                Syscall::encode_result(Ok(SyscallReturn::Value(0)), cpu);
                return Ok(VirtAddr::new(cpu.pc + 4));
            }
            read_buf.push(val as u8);

            // 2. Non-blocking read for subsequent bytes
            let limit = len.min(1024);
            while read_buf.len() < limit {
                let lsr = memory
                    .read_word(PhysAddr::new(UART_BASE + UART_LSR_OFFSET))
                    .map_err(|e| {
                        TrapError::HandlerPanic(format!("UART LSR read error: {:?}", e))
                    })?;

                if (lsr & 0x01) == 0 {
                    break; // No more data
                }

                let val = memory
                    .read_word(PhysAddr::new(UART_BASE + UART_RBR_OFFSET))
                    .map_err(|e| TrapError::HandlerPanic(format!("UART read error: {:?}", e)))?;

                if val == 0 {
                    break; // EOF
                }
                read_buf.push(val as u8);

                if val == 10 || val == 13 {
                    break; // Newline
                }
            }

            // 3. Copy to user
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic("No current thread".into()))?;

            let satp = thread_manager
                .threads
                .get(&current_handle)
                .unwrap()
                .context
                .satp;

            copy_to_user(memory, satp, &read_buf, buf_ptr)?;

            Syscall::encode_result(Ok(SyscallReturn::Value(read_buf.len() as i64)), cpu);
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::BlockRead { sector, buf_ptr } => {
            let mut buffer = [0u8; 512];
            match crate::fs::block::read_sector(memory, sector, &mut buffer) {
                Ok(_) => {
                    let current_handle = thread_manager
                        .current_thread
                        .ok_or(TrapError::HandlerPanic("No current thread".into()))?;
                    let satp = thread_manager
                        .threads
                        .get(&current_handle)
                        .unwrap()
                        .context
                        .satp;

                    copy_to_user(memory, satp, &buffer, buf_ptr)?;
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                }
                Err(_) => {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                }
            }
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::FileOpen { path_ptr, path_len } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "FileOpen: No current thread".into(),
                ))?;

            let satp = thread_manager
                .threads
                .get(&current_handle)
                .unwrap()
                .context
                .satp;

            let mut path_bytes = vec![0u8; path_len];
            copy_from_user(memory, satp, path_ptr, &mut path_bytes)?;

            let path_str = String::from_utf8(path_bytes)
                .map_err(|_| TrapError::HandlerPanic("Invalid UTF-8 path".into()))?;

            let inode_id = if let Some(fs) = file_system {
                fs.find_inode(memory, &path_str)
                    .map_err(|_| SyscallError::InvalidSyscallNumber(0))
            } else {
                Err(SyscallError::InvalidSyscallNumber(0))
            };

            match inode_id {
                Ok(id) => {
                    let tcb = thread_manager.threads.get_mut(&current_handle).unwrap();
                    // Find free FD
                    let fd_idx = tcb.file_descriptors.len();
                    tcb.file_descriptors.push(Some(FileDescriptor::File {
                        inode_id: id,
                        offset: 0,
                        flags: 0,
                    }));
                    Syscall::encode_result(Ok(SyscallReturn::Handle(fd_idx as u32)), cpu);
                }
                Err(_) => {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                }
            }
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::FileRead { fd, buf_ptr, len } => {
            debug!("FileRead: fd={}, buf={:?}, len={}", fd, buf_ptr, len);
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "FileRead: No current thread".into(),
                ))?;

            let (descriptor, satp) = {
                let tcb = thread_manager.threads.get(&current_handle).unwrap();
                let desc = if (fd as usize) < tcb.file_descriptors.len() {
                    tcb.file_descriptors[fd as usize]
                } else {
                    None
                };
                (desc, tcb.context.satp)
            };

            if let Some(FileDescriptor::Pipe { pipe_id, is_write }) = descriptor {
                if is_write {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                    return Ok(VirtAddr::new(cpu.pc + 4));
                }

                let mut should_block = false;
                let mut result_val = 0;

                if let Some(pipe) = pipes.get_mut(&pipe_id) {
                    if pipe.buffer.is_empty() {
                        if !pipe.write_open {
                            // EOF
                            result_val = 0;
                        } else {
                            // Block
                            pipe.wait_queue.push_back(current_handle);
                            should_block = true;
                        }
                    } else {
                        // Read available data
                        let mut read_bytes = Vec::new();
                        while read_bytes.len() < len {
                            if let Some(b) = pipe.buffer.pop_front() {
                                read_bytes.push(b);
                            } else {
                                break;
                            }
                        }
                        copy_to_user(memory, satp, &read_bytes, buf_ptr)?;
                        result_val = read_bytes.len() as i64;
                    }
                } else {
                    // Invalid Pipe
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                    return Ok(VirtAddr::new(cpu.pc + 4));
                }

                if should_block {
                    thread_manager.block_current_thread();
                    if !thread_manager.yield_thread(cpu) {
                        return Err(TrapError::Halt);
                    }
                    // Do not advance PC, retry syscall when woken
                    Ok(VirtAddr::new(cpu.pc))
                } else {
                    Syscall::encode_result(Ok(SyscallReturn::Value(result_val)), cpu);
                    Ok(VirtAddr::new(cpu.pc + 4))
                }
            } else if let Some(FileDescriptor::File {
                inode_id,
                offset,
                flags: _,
            }) = descriptor
            {
                if let Some(fs) = file_system {
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

                        if bytes == 0 {
                            break;
                        }

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

                    let tcb = thread_manager.threads.get_mut(&current_handle).unwrap();
                    if let Some(Some(FileDescriptor::File { offset, .. })) =
                        tcb.file_descriptors.get_mut(fd as usize)
                    {
                        *offset = current_offset;
                    }

                    Syscall::encode_result(Ok(SyscallReturn::Value(total_read as i64)), cpu);
                } else {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            } else {
                // Invalid FD
                Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                Ok(VirtAddr::new(cpu.pc + 4))
            }
        }

        Syscall::FileClose { fd } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "FileClose: No current thread".into(),
                ))?;

            let closed_fd = {
                let tcb = thread_manager.threads.get_mut(&current_handle).unwrap();
                if (fd as usize) < tcb.file_descriptors.len() {
                    tcb.file_descriptors[fd as usize].take()
                } else {
                    None
                }
            };

            match closed_fd {
                Some(FileDescriptor::Pipe { pipe_id, is_write }) => {
                    let mut should_remove = false;
                    let mut to_wake = Vec::new();

                    if let Some(pipe) = pipes.get_mut(&pipe_id) {
                        if is_write {
                            pipe.write_open = false;
                            // Wake readers for EOF
                            while let Some(h) = pipe.wait_queue.pop_front() {
                                to_wake.push(h);
                            }
                        } else {
                            pipe.read_open = false;
                        }

                        if !pipe.read_open && !pipe.write_open {
                            should_remove = true;
                        }
                    }

                    if should_remove {
                        pipes.remove(&pipe_id);
                    }

                    for h in to_wake {
                        thread_manager.wake_thread(h);
                    }

                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                }
                Some(_) => {
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                }
                None => {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                }
            }
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        _ => Err(TrapError::HandlerPanic("FS: Unhandled syscall".into())),
    }
}
