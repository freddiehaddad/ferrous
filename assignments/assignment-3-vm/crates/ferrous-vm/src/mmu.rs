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
    // TODO: Assignment 3 - Implement hardware MMU translation logic
    // 1. Check if paging is enabled (satp.MODE).
    // 2. Extract PPN from SATP.
    // 3. Walk L1 Page Table (Check valid, leaf, permissions).
    // 4. Walk L0 Page Table (Check valid, permissions).
    // 5. Check permissions (User/Supervisor, R/W/X).
    // 6. Return physical address or raise TrapCause (PageFault/AccessFault).
    todo!("Assignment 3: MMU Translation");
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
