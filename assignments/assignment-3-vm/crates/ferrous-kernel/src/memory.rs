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
    // TODO: Assignment 3/4 - Implement page mapping
    // 1. Calculate VPN1 and VPN0.
    // 2. Traverse/Create L1 Page Table.
    // 3. Create L0 Page Table if needed.
    // 4. Update L0 PTE with PPN and Flags.
    todo!("Assignment 3: map_page");
}

/// Sets up the initial kernel address space (Identity Mapping).
/// This is needed so the kernel can run with virtual memory enabled.
pub fn setup_kernel_address_space(memory: &mut dyn Memory) -> Result<u32, String> {
    debug!("Setting up Kernel Address Space...");

    // TODO: Assignment 3 - Setup Kernel Address Space
    // 1. Allocate root page table.
    // 2. Identity map Kernel Code/Data.
    // 3. Identity map MMIO regions (UART, Block Device).
    // 4. Map Kernel Stack? (Optional if covered by step 2).
    // 5. Return SATP value.
    todo!("Assignment 3: setup_kernel_address_space");
}

/// Creates a new page table for a user process.
///
/// **Assignment 4:** Students will implement full address space creation,
/// ensuring isolation between kernel and user space.
pub fn create_user_address_space(memory: &mut dyn Memory) -> Result<u32, String> {
    // TODO: Assignment 4 - Create User Address Space
    // 1. Allocate root page table.
    // 2. Map necessary kernel/IO regions (Trampoline?).
    // 3. Return SATP.
    todo!("Assignment 4: create_user_address_space");
}

/// Translates a virtual address to a physical address using the page table (SATP).
/// This is crucial for accessing user memory from the kernel.
pub fn translate_vaddr(memory: &mut dyn Memory, satp: u32, vaddr: u32) -> Result<u32, TrapError> {
    // TODO: Assignment 3 - Implement software page walker
    // 1. Extract PPN from SATP.
    // 2. Walk L1 table.
    // 3. Walk L0 table.
    // 4. Return physical address.
    todo!("Assignment 3: translate_vaddr");
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
