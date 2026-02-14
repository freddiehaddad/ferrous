use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use ferrous_vm::{Memory, PhysAddr, TrapError, VirtAddr};
use goblin::elf;
use log::info;

use crate::memory::{self, copy_to_user, translate_vaddr};
use crate::thread::ThreadManager;

pub mod syscalls;

/// Bootstraps the first user process from an ELF binary.
///
/// This function:
/// 1. Creates a new Address Space (Page Table).
/// 2. Parses the ELF header and loads segments (Code, Data) into memory.
/// 3. Sets up the User Stack with arguments (argc, argv).
/// 4. Creates the initial Thread.
///
/// **Assignment 5:** Students will implement `fork` and `exec` which reuse much of this logic.
pub fn bootstrap_process(
    thread_manager: &mut ThreadManager,
    memory: &mut dyn Memory,
    elf_data: &[u8],
    args: &[String],
) -> Result<(VirtAddr, u32, u32, u32, u32), TrapError> {
    let elf = elf::Elf::parse(elf_data)
        .map_err(|e| TrapError::HandlerPanic(format!("Bootstrap: Invalid ELF: {:?}", e)))?;

    // 1. Create Address Space
    let satp_val = memory::create_user_address_space(memory).map_err(TrapError::HandlerPanic)?;
    let root_ppn = satp_val & 0x003F_FFFF;

    // 2. Load Segments
    // Iterates through Program Headers (PH) and loads PT_LOAD segments
    // TODO: This manual loading logic is what `exec` will need (Assignment 5)
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

            // Map pages and copy data
            while current_vaddr < end_vaddr {
                let page_base = current_vaddr & !(memory::PAGE_SIZE - 1);

                // Translate to see if page already exists, else allocate
                let paddr_base = match translate_vaddr(memory, satp_val, page_base) {
                    Ok(p) => p & !(memory::PAGE_SIZE - 1),
                    Err(_) => {
                        let frame = memory::alloc_frame();
                        let flags = memory::PTE_R | memory::PTE_W | memory::PTE_U | memory::PTE_X;
                        memory::map_page(memory, root_ppn, page_base, frame, flags)
                            .map_err(TrapError::HandlerPanic)?;

                        // Zero fill new page
                        for i in 0..memory::PAGE_SIZE {
                            memory.write_byte(PhysAddr::new(frame + i), 0).unwrap();
                        }
                        frame
                    }
                };

                // Calculate copy bounds for this page
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
                                TrapError::HandlerPanic(format!("Bootstrap write error: {:?}", e))
                            })?;
                    }
                }
                current_vaddr += chunk_size;
            }
        }
    }

    // 3. Setup Stack
    // Allocates fixed size stack at 0xF000_0000
    // TODO: Dynamic stack growth (Assignment 4/5)
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
        current_sp -= arg_bytes.len() as u32;
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
    let handle = thread_manager
        .create_thread(entry_point, current_sp)
        .map_err(TrapError::HandlerPanic)?;

    if let Some(tcb) = thread_manager.threads.get_mut(&handle) {
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

    thread_manager.current_thread = Some(handle);

    Ok((
        entry_point,
        satp_val,
        current_sp,
        args.len() as u32,
        argv_base,
    ))
}
