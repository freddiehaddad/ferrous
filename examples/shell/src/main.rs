#![no_std]
#![no_main]

extern crate alloc;
extern crate ferrous_fs;
extern crate ferrous_user;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use ferrous_fs::DirEntry;
use ferrous_user::syscall;
use ferrous_user::{print, println};

struct SbrkAllocator;

unsafe impl GlobalAlloc for SbrkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Get current break
        let current_break = syscall::sbrk(0) as usize;

        // Calculate required alignment padding
        let padding = (align - (current_break % align)) % align;
        let total_size = size + padding;

        // Allocate
        let start = syscall::sbrk(total_size as i32) as usize;
        if start == 0 {
            return core::ptr::null_mut();
        }

        (start + padding) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op
    }
}

#[global_allocator]
static ALLOCATOR: SbrkAllocator = SbrkAllocator;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn test_pipe() {
    println!("Testing pipes...");
    let mut fds = [0u32; 2];
    match syscall::pipe(&mut fds) {
        Ok(_) => {
            let read_fd = fds[0];
            let write_fd = fds[1];
            println!("Pipe created. Read FD: {}, Write FD: {}", read_fd, write_fd);

            // Create a child thread (acting as a "process" sharing address space for now)
            // Note: Since we don't have fork(), threads share memory.
            // But FDs are per-process usually?
            // Ferrous Reference Implementation: Threads share the same file descriptor table (process-wide).
            // So we can write in one thread and read in another.

            // To properly test, we need concurrency.
            alloc_stack();
            // We need to pass arguments to the thread.

            // Ferrous `thread_create` only takes entry and stack.
            // We'll use a global or static for coordination in this simple test,
            // OR just write then read in the same thread to verify basic buffering.

            // 1. Basic Write/Read Test (Same Thread)
            let msg = "Hello from pipe!";
            println!("Writing to pipe: '{}'", msg);
            match syscall::file_write(write_fd, msg.as_bytes()) {
                Ok(n) => println!("Wrote {} bytes", n),
                Err(e) => println!("Write failed: {}", e),
            }

            let mut buf = [0u8; 32];
            match syscall::file_read(read_fd, &mut buf) {
                Ok(n) => {
                    if let Ok(s) = core::str::from_utf8(&buf[0..n]) {
                        println!("Read from pipe: '{}'", s);
                    } else {
                        println!("Read invalid UTF-8");
                    }
                }
                Err(e) => println!("Read failed: {}", e),
            }

            syscall::file_close(write_fd);
            syscall::file_close(read_fd);
        }
        Err(e) => println!("Pipe creation failed: {}", e),
    }
}

// Helper for allocating stack for threads (copied from lib)
fn alloc_stack() -> usize {
    let stack_size = 4096;
    let stack_bottom = syscall::sbrk(stack_size as i32) as usize;
    stack_bottom + stack_size
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    ferrous_user::init();

    println!("Ferrous OS Shell v0.1");
    println!("Type 'help' for commands.");

    let mut buf = [0u8; 128];
    let mut cursor = 0;
    print!("> ");

    loop {
        // Read into buffer at current cursor
        let n = syscall::console_read(0, &mut buf[cursor..]);
        if n == 0 {
            break;
        }

        // Check for newline in new data
        let mut newline_idx = None;
        for i in 0..n {
            if buf[cursor + i] == b'\n' || buf[cursor + i] == b'\r' {
                newline_idx = Some(cursor + i);
                break;
            }
        }

        cursor += n;

        if let Some(idx) = newline_idx {
            // Process command
            let cmd_bytes = &buf[0..idx];
            if let Ok(cmd_str) = core::str::from_utf8(cmd_bytes) {
                // Parse command respecting quotes
                let mut parts: Vec<&str> = Vec::new();
                let mut in_quote = false;
                let mut start = 0;
                let chars = cmd_str.char_indices();

                for (i, c) in chars {
                    if c == '"' {
                        if in_quote {
                            // End quote
                            parts.push(&cmd_str[start..i]);
                            in_quote = false;
                            start = i + 1;
                        } else {
                            // Start quote
                            if i > start {
                                parts.push(&cmd_str[start..i]);
                            }
                            in_quote = true;
                            start = i + 1;
                        }
                    } else if c.is_whitespace() && !in_quote {
                        if i > start {
                            parts.push(&cmd_str[start..i]);
                        }
                        start = i + 1;
                    }
                }
                if start < cmd_str.len() {
                    parts.push(&cmd_str[start..]);
                }

                // Filter empty parts
                let parts: Vec<&str> = parts.into_iter().filter(|s| !s.is_empty()).collect();

                if !parts.is_empty() {
                    let program = parts[0];
                    // Copy args to stack array to avoid passing heap pointer to syscall (workaround for kernel/user heap issue)
                    let mut args_buf = [""; 32];
                    let args_count = parts.len() - 1;
                    if args_count > 32 {
                        println!("Too many arguments (max 32)");
                        continue;
                    }
                    args_buf[..args_count].copy_from_slice(&parts[1..(args_count + 1)]);
                    let args = &args_buf[0..args_count];

                    if program == "ls" {
                        match syscall::file_open("/") {
                            Ok(fd) => {
                                let entry_size = core::mem::size_of::<DirEntry>();
                                let mut dir_buf = [0u8; 512];
                                loop {
                                    match syscall::file_read(fd, &mut dir_buf) {
                                        Ok(bytes_read) => {
                                            if bytes_read == 0 {
                                                break;
                                            }
                                            let count = bytes_read / entry_size;
                                            for i in 0..count {
                                                let offset = i * entry_size;
                                                let entry_ptr = unsafe {
                                                    dir_buf.as_ptr().add(offset) as *const DirEntry
                                                };
                                                let entry = unsafe { entry_ptr.read_unaligned() };
                                                if entry.name[0] == 0 {
                                                    continue;
                                                }
                                                println!("{}", entry.name_as_str());
                                            }
                                        }
                                        Err(_) => {
                                            println!("Error reading directory.");
                                            break;
                                        }
                                    }
                                }
                                syscall::file_close(fd);
                            }
                            Err(_) => {
                                println!("Failed to open root directory.");
                            }
                        }
                    } else if program == "cat" {
                        if !args.is_empty() {
                            let filename = args[0];
                            match syscall::file_open(filename) {
                                Ok(fd) => {
                                    let mut file_buf = [0u8; 128];
                                    loop {
                                        match syscall::file_read(fd, &mut file_buf) {
                                            Ok(0) => break,
                                            Ok(n) => {
                                                syscall::console_write(1, &file_buf[0..n]);
                                            }
                                            Err(_) => {
                                                println!("Error reading file.");
                                                break;
                                            }
                                        }
                                    }
                                    println!("");
                                    syscall::file_close(fd);
                                }
                                Err(_) => {
                                    println!("Failed to open file: {}", filename);
                                }
                            }
                        } else {
                            println!("Usage: cat <file>");
                        }
                    } else if program == "help" {
                        println!("Available commands:");
                        println!("  ls          - List files");
                        println!("  cat <file>  - Display file contents");
                        println!("  pipe_test   - Run pipe test");
                        println!("  exit        - Quit shell");
                        println!("  help        - Show this message");
                        println!("");
                        println!("External programs:");
                        println!("  echo <args> - Print arguments");
                        println!("  threads     - Multithreading demo");
                        println!("  sbrk        - Heap allocation demo");
                        println!("  hello       - Hello World");
                        println!("  file-read   - File reading demo");
                    } else if program == "pipe_test" {
                        test_pipe();
                    } else if program == "exit" {
                        println!("Goodbye!");
                        break;
                    } else {
                        match syscall::exec(program, args) {
                            Ok(handle) => {
                                // println!("Spawned process {}", handle);
                                let _code = syscall::waitpid(handle);
                                // println!("Process {} exited with code {}", handle, code);
                            }
                            Err(_) => {
                                println!("Unknown command or executable not found: {}", program);
                            }
                        }
                    }
                }
            }

            // Reset buffer
            cursor = 0;
            print!("> ");
        }

        if cursor >= buf.len() {
            println!("Line too long");
            cursor = 0;
            print!("> ");
        }
    }

    ferrous_user::exit(0);
}
