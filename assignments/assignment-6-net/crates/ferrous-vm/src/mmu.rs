use crate::cpu::PrivilegeMode;
use crate::memory::{Memory, PhysAddr, VirtAddr};
use crate::trap::TrapCause;

pub const PAGE_SIZE: u32 = 4096;

// Page Table Entry Flags
pub const PTE_V: u32 = 1 << 0; // Valid
pub const PTE_R: u32 = 1 << 1; // Read
pub const PTE_W: u32 = 1 << 2; // Write
pub const PTE_X: u32 = 1 << 3; // Execute
pub const PTE_U: u32 = 1 << 4; // User
pub const PTE_G: u32 = 1 << 5; // Global
pub const PTE_A: u32 = 1 << 6; // Accessed
pub const PTE_D: u32 = 1 << 7; // Dirty

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

pub fn translate(
    addr: VirtAddr,
    access_type: AccessType,
    satp: u32,
    mode: PrivilegeMode,
    memory: &mut dyn Memory,
) -> Result<PhysAddr, TrapCause> {
    // 1. Check if paging is enabled (satp.MODE = 1 for SV32)
    // For SV32, MODE is bit 31.
    if satp & 0x8000_0000 == 0 || mode == PrivilegeMode::Machine {
        return Ok(PhysAddr::new(addr.val()));
    }

    let ppn_root = satp & 0x003F_FFFF; // Bits 0-21
    let vpn1 = (addr.val() >> 22) & 0x3FF;
    let vpn0 = (addr.val() >> 12) & 0x3FF;
    let offset = addr.val() & 0xFFF;

    // First Level
    let pte_addr_1 = PhysAddr::new((ppn_root << 12) + (vpn1 * 4));
    let pte1 = memory
        .read_word(pte_addr_1)
        .map_err(|_| TrapCause::LoadAccessFault { addr })?;

    if pte1 & PTE_V == 0 {
        return Err(match access_type {
            AccessType::Read => TrapCause::LoadPageFault { addr },
            AccessType::Write => TrapCause::StorePageFault { addr },
            AccessType::Execute => TrapCause::InstructionPageFault { addr },
        });
    }

    // Check if leaf (R, W, or X set)
    if (pte1 & (PTE_R | PTE_W | PTE_X)) != 0 {
        // Superpage (4MB)
        // Check alignment for superpage
        if (pte1 >> 10) & 0x3FF != 0 {
            // Misaligned superpage
            return Err(match access_type {
                AccessType::Read => TrapCause::LoadPageFault { addr },
                AccessType::Write => TrapCause::StorePageFault { addr },
                AccessType::Execute => TrapCause::InstructionPageFault { addr },
            });
        }

        // Use PPN[1] from PTE, PPN[0] from VA
        let ppn1 = (pte1 >> 20) & 0xFFF;
        let pa = (ppn1 << 22) | (vpn0 << 12) | offset;

        check_permissions(pte1, access_type, mode, addr)?;

        return Ok(PhysAddr::new(pa));
    }

    // Pointer to next level
    let ppn1 = (pte1 >> 10) & 0x3F_FFFF; // Bits 10-31

    // Second Level
    let pte_addr_0 = PhysAddr::new((ppn1 << 12) + (vpn0 * 4));
    let pte0 = memory
        .read_word(pte_addr_0)
        .map_err(|_| TrapCause::LoadAccessFault { addr })?;

    if pte0 & PTE_V == 0 {
        return Err(match access_type {
            AccessType::Read => TrapCause::LoadPageFault { addr },
            AccessType::Write => TrapCause::StorePageFault { addr },
            AccessType::Execute => TrapCause::InstructionPageFault { addr },
        });
    }

    // Leaf check
    if (pte0 & (PTE_R | PTE_W | PTE_X)) == 0 {
        // Invalid PTE (neither leaf nor pointer at level 0)
        return Err(match access_type {
            AccessType::Read => TrapCause::LoadPageFault { addr },
            AccessType::Write => TrapCause::StorePageFault { addr },
            AccessType::Execute => TrapCause::InstructionPageFault { addr },
        });
    }

    check_permissions(pte0, access_type, mode, addr)?;

    // Construct PA
    let ppn0 = (pte0 >> 10) & 0x3F_FFFF;
    let pa = (ppn0 << 12) | offset;
    Ok(PhysAddr::new(pa))
}

fn check_permissions(
    pte: u32,
    access_type: AccessType,
    mode: PrivilegeMode,
    addr: VirtAddr,
) -> Result<(), TrapCause> {
    // Privilege check
    match mode {
        PrivilegeMode::User => {
            if pte & PTE_U == 0 {
                return Err(match access_type {
                    AccessType::Read => TrapCause::LoadPageFault { addr },
                    AccessType::Write => TrapCause::StorePageFault { addr },
                    AccessType::Execute => TrapCause::InstructionPageFault { addr },
                });
            }
        }
        PrivilegeMode::Supervisor => {
            if pte & PTE_U != 0 {
                // S-mode cannot access U-mode pages (unless SUM is set, assume 0)
                return Err(match access_type {
                    AccessType::Read => TrapCause::LoadPageFault { addr },
                    AccessType::Write => TrapCause::StorePageFault { addr },
                    AccessType::Execute => TrapCause::InstructionPageFault { addr },
                });
            }
        }
        PrivilegeMode::Machine => {} // M-mode bypasses translation usually, but if we are here...
    }

    // Access Type check
    let valid = match access_type {
        AccessType::Read => (pte & PTE_R) != 0 || (pte & PTE_X) != 0, // MXR bit (assume 1 for simplicity or strictly follow R) - R=1 is strict.
        AccessType::Write => (pte & PTE_W) != 0,
        AccessType::Execute => (pte & PTE_X) != 0,
    };

    if !valid {
        return Err(match access_type {
            AccessType::Read => TrapCause::LoadPageFault { addr },
            AccessType::Write => TrapCause::StorePageFault { addr },
            AccessType::Execute => TrapCause::InstructionPageFault { addr },
        });
    }

    Ok(())
}
