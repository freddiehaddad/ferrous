use crate::syscall::{Syscall, SyscallReturn};
use crate::thread::ThreadManager;
use alloc::format;
use alloc::string::String;
use ferrous_vm::{Cpu, Memory, PhysAddr, TrapError, VirtAddr};
use log::debug;

// Page Size (4KB)
pub const PAGE_SIZE: u32 = 4096;

// Page Table Entry (PTE) Flags
// RISC-V Sv32 Page Table Entry format
pub const PTE_V: u32 = 1 << 0; // Valid
pub const PTE_R: u32 = 1 << 1; // Read
pub const PTE_W: u32 = 1 << 2; // Write
pub const PTE_X: u32 = 1 << 3; // Execute
pub const PTE_U: u32 = 1 << 4; // User accessible
pub const PTE_G: u32 = 1 << 5; // Global
pub const PTE_A: u32 = 1 << 6; // Accessed
pub const PTE_D: u32 = 1 << 7; // Dirty

// SATP Mode (SV32 - 2-level page table)
pub const SATP_MODE_SV32: u32 = 1 << 31;

// Simple Bump Allocator for Frames (Physical Memory)
// Start after Kernel code/data (assuming 4MB for Kernel)
// TODO: Replace with a proper Frame Allocator (Bitmap or Free List) in Assignment 4
static mut NEXT_FREE_FRAME: u32 = 0x8040_0000;

/// Allocates a single physical frame (4KB).
/// Currently uses a bump pointer allocator.
pub fn alloc_frame() -> u32 {
    unsafe {
        let addr = NEXT_FREE_FRAME;
        NEXT_FREE_FRAME += PAGE_SIZE;
        addr
    }
}

/// Maps a virtual page to a physical frame in the page table.
///
/// **Assignment 4:** Students will implement or extend this function to support
/// complex mapping scenarios and permission checks.
pub fn map_page(
    memory: &mut dyn Memory,
    root_ppn: u32,
    vaddr: u32,
    paddr: u32,
    flags: u32,
) -> Result<(), String> {
    let vpn1 = (vaddr >> 22) & 0x3FF;
    let vpn0 = (vaddr >> 12) & 0x3FF;

    // L1 Page Table Access (Root)
    let l1_pte_addr = PhysAddr::new((root_ppn << 12) + (vpn1 * 4));
    let mut l1_pte = memory
        .read_word(l1_pte_addr)
        .map_err(|e| format!("Failed to read L1 PTE: {:?}", e))?;

    if (l1_pte & PTE_V) == 0 {
        // Allocate L0 Page Table if missing
        let l0_table_pa = alloc_frame();
        // Zero out the new page table (critical for security)
        for i in 0..1024 {
            memory
                .write_word(PhysAddr::new(l0_table_pa + i * 4), 0)
                .map_err(|e| format!("Failed to zero L0 PTE: {:?}", e))?;
        }

        // Create Valid PTE pointing to L0 table
        let l0_ppn = l0_table_pa >> 12;
        l1_pte = (l0_ppn << 10) | PTE_V;
        memory
            .write_word(l1_pte_addr, l1_pte)
            .map_err(|e| format!("Failed to write L1 PTE: {:?}", e))?;
    }

    // L0 Page Table Access (Leaf)
    let l0_ppn = (l1_pte >> 10) & 0x3F_FFFF;
    let l0_pte_addr = PhysAddr::new((l0_ppn << 12) + (vpn0 * 4));

    let ppn = paddr >> 12;
    let l0_pte = (ppn << 10) | flags | PTE_V | PTE_A | PTE_D; // Pre-set Accessed/Dirty for simplicity

    memory
        .write_word(l0_pte_addr, l0_pte)
        .map_err(|e| format!("Failed to write L0 PTE: {:?}", e))?;

    Ok(())
}

/// Sets up the initial kernel address space (Identity Mapping).
/// This is needed so the kernel can run with virtual memory enabled.
pub fn setup_kernel_address_space(memory: &mut dyn Memory) -> Result<u32, String> {
    debug!("Setting up Kernel Address Space...");

    // Allocate Root Page Table
    let root_pa = alloc_frame();
    // Zero root table
    for i in 0..1024 {
        memory
            .write_word(PhysAddr::new(root_pa + i * 4), 0)
            .map_err(|e| format!("Failed to zero root PTE: {:?}", e))?;
    }
    let root_ppn = root_pa >> 12;

    // 1. Identity Map Kernel Code/Data (0x8000_0000 - 0x8040_0000)
    // Map 4MB (1024 pages)
    let kernel_start = 0x8000_0000;
    for i in 0..1024 {
        let addr = kernel_start + (i * PAGE_SIZE);
        map_page(
            memory,
            root_ppn,
            addr,
            addr,
            PTE_R | PTE_W | PTE_X, // RWX for simplicity
        )?;
    }

    // 2. Identity Map MMIO (UART at 0x1000_0000)
    // Map 1 Page
    let uart_addr = 0x1000_0000;
    map_page(
        memory,
        root_ppn,
        uart_addr,
        uart_addr,
        PTE_R | PTE_W, // RW (No Execute)
    )?;

    // 3. Identity Map Block Device MMIO (at 0x2000_0000)
    // Map 1 Page (SimpleBlockDevice uses 0x1000 size)
    let block_addr = 0x2000_0000;
    map_page(memory, root_ppn, block_addr, block_addr, PTE_R | PTE_W)?;

    // 4. Stack Mapping for Initial Process
    // Map top 64KB of RAM (0x80FF_0000 - 0x8100_0000)
    // This assumes 16MB RAM
    let stack_start = 0x80FF_0000;
    for i in 0..16 {
        let addr = stack_start + (i * PAGE_SIZE);
        map_page(memory, root_ppn, addr, addr, PTE_R | PTE_W)?;
    }

    debug!(
        "Kernel Address Space initialized. Root PPN: {:#x}",
        root_ppn
    );

    // Return SATP value (Mode=SV32, PPN=root_ppn)
    Ok(SATP_MODE_SV32 | root_ppn)
}

/// Creates a new page table for a user process.
///
/// **Assignment 4:** Students will implement full address space creation,
/// ensuring isolation between kernel and user space.
pub fn create_user_address_space(memory: &mut dyn Memory) -> Result<u32, String> {
    // Allocate Root Page Table
    let root_pa = alloc_frame();
    // Zero root table
    for i in 0..1024 {
        memory
            .write_word(PhysAddr::new(root_pa + i * 4), 0)
            .map_err(|e| format!("Failed to zero root PTE: {:?}", e))?;
    }
    let root_ppn = root_pa >> 12;

    // Map Kernel/IO regions (Must match setup_kernel_address_space for trap handlers)
    // 1. REMOVED: Kernel Code/Data (0x8000_0000)
    // We don't want to map this for user processes as they might load their own code at 0x8000_0000.
    // Since the kernel is external (host-based), we don't need to protect kernel code in VM memory.

    // 2. UART
    let uart_addr = 0x1000_0000;
    map_page(memory, root_ppn, uart_addr, uart_addr, PTE_R | PTE_W)?;

    // 3. Block Device
    let block_addr = 0x2000_0000;
    map_page(memory, root_ppn, block_addr, block_addr, PTE_R | PTE_W)?;

    Ok(SATP_MODE_SV32 | root_ppn)
}

/// Translates a virtual address to a physical address using the page table (SATP).
/// This is crucial for accessing user memory from the kernel.
pub fn translate_vaddr(memory: &mut dyn Memory, satp: u32, vaddr: u32) -> Result<u32, TrapError> {
    // Check Mode (MSB of SATP)
    // If Mode is 0, Bare mode (Physical = Virtual)
    if (satp & 0x8000_0000) == 0 {
        return Ok(vaddr);
    }

    let root_ppn = satp & 0x003F_FFFF;
    let vpn1 = (vaddr >> 22) & 0x3FF;
    let vpn0 = (vaddr >> 12) & 0x3FF;
    let offset = vaddr & 0xFFF;

    let l1_pte_addr = PhysAddr::new((root_ppn << 12) + (vpn1 * 4));
    let l1_pte = memory
        .read_word(l1_pte_addr)
        .map_err(|e| TrapError::HandlerPanic(format!("L1 read error: {:?}", e)))?;

    if (l1_pte & PTE_V) == 0 {
        return Err(TrapError::HandlerPanic("Page fault (L1 invalid)".into()));
    }

    let l0_ppn = (l1_pte >> 10) & 0x3F_FFFF;
    let l0_pte_addr = PhysAddr::new((l0_ppn << 12) + (vpn0 * 4));
    let l0_pte = memory
        .read_word(l0_pte_addr)
        .map_err(|e| TrapError::HandlerPanic(format!("L0 read error: {:?}", e)))?;

    if (l0_pte & PTE_V) == 0 {
        return Err(TrapError::HandlerPanic("Page fault (L0 invalid)".into()));
    }

    let ppn = (l0_pte >> 10) & 0x3F_FFFF;
    let paddr = (ppn << 12) | offset;
    Ok(paddr)
}

/// Safely copies data from user space to kernel space.
pub fn copy_from_user(
    memory: &mut dyn Memory,
    satp: u32,
    src_ptr: VirtAddr,
    dest: &mut [u8],
) -> Result<(), TrapError> {
    for (i, byte) in dest.iter_mut().enumerate() {
        let vaddr = src_ptr.val() + i as u32;
        let paddr = translate_vaddr(memory, satp, vaddr)?;
        *byte = memory
            .read_byte(PhysAddr::new(paddr))
            .map_err(|e| TrapError::HandlerPanic(format!("User read error: {:?}", e)))?;
    }
    Ok(())
}

/// Safely copies data from kernel space to user space.
pub fn copy_to_user(
    memory: &mut dyn Memory,
    satp: u32,
    src: &[u8],
    dest_ptr: VirtAddr,
) -> Result<(), TrapError> {
    for (i, byte) in src.iter().enumerate() {
        let vaddr = dest_ptr.val() + i as u32;
        let paddr = translate_vaddr(memory, satp, vaddr)?;
        memory
            .write_byte(PhysAddr::new(paddr), *byte)
            .map_err(|e| TrapError::HandlerPanic(format!("User write error: {:?}", e)))?;
    }
    Ok(())
}

/// Handles the `sbrk` system call (increase heap size).
///
/// **Assignment 4:** Students will implement `brk` or `mmap` for more advanced memory management.
pub fn handle_sbrk(
    increment: i32,
    thread_manager: &mut ThreadManager,
    memory: &mut dyn Memory,
    cpu: &mut Cpu,
) -> Result<VirtAddr, TrapError> {
    let current_handle = thread_manager
        .current_thread
        .ok_or(TrapError::HandlerPanic(
            "Sbrk called without current thread".into(),
        ))?;

    // Get current program break
    let mut current_break = 0;
    let mut root_ppn = 0;

    if let Some(tcb) = thread_manager.threads.get(&current_handle) {
        current_break = tcb.program_break;
        root_ppn = tcb.context.satp & 0x003F_FFFF; // Extract PPN from SATP
    }

    if increment == 0 {
        Syscall::encode_result(Ok(SyscallReturn::Value(current_break as i64)), cpu);
        return Ok(VirtAddr::new(cpu.pc + 4));
    }

    let new_break = (current_break as i32 + increment) as u32;

    // Align to page boundary for mapping check
    let old_page_end = (current_break + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    let new_page_end = (new_break + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    if increment > 0 {
        // Growing
        if new_page_end > old_page_end {
            // Need to allocate new pages
            let start_page = old_page_end;
            let end_page = new_page_end;
            let mut page_addr = start_page;

            debug!(
                "Sbrk: Allocating {} bytes. Old break: {:#x}. Mapping pages from {:#x} to {:#x}",
                increment, current_break, start_page, end_page
            );

            while page_addr < end_page {
                // Alloc frame
                let frame = alloc_frame();
                // Map
                map_page(
                    memory,
                    root_ppn,
                    page_addr,
                    frame,
                    PTE_R | PTE_W | PTE_U, // User RW
                )
                .map_err(TrapError::HandlerPanic)?;

                page_addr += PAGE_SIZE;
            }
        }
    } else {
        // Shrinking (Not implemented yet for safety/simplicity, just update break)
    }

    // Update TCB
    if let Some(tcb) = thread_manager.threads.get_mut(&current_handle) {
        tcb.program_break = new_break;
    }

    Syscall::encode_result(Ok(SyscallReturn::Value(current_break as i64)), cpu);
    Ok(VirtAddr::new(cpu.pc + 4))
}
