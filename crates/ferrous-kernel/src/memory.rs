use alloc::format;
use alloc::string::String;
use ferrous_vm::{Memory, PhysAddr};
use log::debug;

// Page Size
pub const PAGE_SIZE: u32 = 4096;

// PTE Flags
pub const PTE_V: u32 = 1 << 0;
pub const PTE_R: u32 = 1 << 1;
pub const PTE_W: u32 = 1 << 2;
pub const PTE_X: u32 = 1 << 3;
pub const PTE_U: u32 = 1 << 4;
pub const PTE_G: u32 = 1 << 5;
pub const PTE_A: u32 = 1 << 6;
pub const PTE_D: u32 = 1 << 7;

// SATP Mode (SV32)
pub const SATP_MODE_SV32: u32 = 1 << 31;

// Simple Bump Allocator for Frames (Physical Memory)
// Start after Kernel (assuming 4MB for Kernel code/data)
static mut NEXT_FREE_FRAME: u32 = 0x8040_0000;

pub fn alloc_frame() -> u32 {
    unsafe {
        let addr = NEXT_FREE_FRAME;
        NEXT_FREE_FRAME += PAGE_SIZE;
        addr
    }
}

pub fn map_page(
    memory: &mut dyn Memory,
    root_ppn: u32,
    vaddr: u32,
    paddr: u32,
    flags: u32,
) -> Result<(), String> {
    let vpn1 = (vaddr >> 22) & 0x3FF;
    let vpn0 = (vaddr >> 12) & 0x3FF;

    // L1 Page Table Access
    let l1_pte_addr = PhysAddr::new((root_ppn << 12) + (vpn1 * 4));
    let mut l1_pte = memory
        .read_word(l1_pte_addr)
        .map_err(|e| format!("Failed to read L1 PTE: {:?}", e))?;

    if (l1_pte & PTE_V) == 0 {
        // Allocate L0 Page Table
        let l0_table_pa = alloc_frame();
        // Zero out the new page table
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

    // L0 Page Table Access
    let l0_ppn = (l1_pte >> 10) & 0x3F_FFFF;
    let l0_pte_addr = PhysAddr::new((l0_ppn << 12) + (vpn0 * 4));

    let ppn = paddr >> 12;
    let l0_pte = (ppn << 10) | flags | PTE_V | PTE_A | PTE_D; // Pre-set Accessed/Dirty for simplicity

    memory
        .write_word(l0_pte_addr, l0_pte)
        .map_err(|e| format!("Failed to write L0 PTE: {:?}", e))?;

    Ok(())
}

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

    // Map Kernel/IO regions (Must match setup_kernel_address_space)

    // 1. REMOVED: Kernel Code/Data (0x8000_0000)
    // We don't want to map this for user processes as they might load their own code at 0x8000_0000.
    // Since the kernel is external (host-based), we don't need to protect kernel code in VM memory.

    // 2. UART
    let uart_addr = 0x1000_0000;
    map_page(memory, root_ppn, uart_addr, uart_addr, PTE_R | PTE_W)?;

    // 3. Block Device
    let block_addr = 0x2000_0000;
    map_page(memory, root_ppn, block_addr, block_addr, PTE_R | PTE_W)?;

    // 4. REMOVED: Physical RAM identity mapping
    // User heap (sbrk) will allocate and map frames dynamically.
    // Pre-mapping conflicts with sbrk's assumption of fresh frames.

    Ok(SATP_MODE_SV32 | root_ppn)
}
