# Ferrous OS Architecture Reference

## 1. System Overview
Ferrous OS is a monolithic kernel targeting the **RISC-V 32-bit (RV32IMAC)** architecture. It runs on the `virt` machine board in QEMU.

### Core Specifications
*   **Architecture:** RISC-V 32-bit (Sv32 Paging).
*   **Privilege Levels:** Machine (M-Mode) for boot/traps, Supervisor (S-Mode) for kernel, User (U-Mode) for applications.
*   **Scheduling:** Preemptive Round-Robin.
*   **Memory Management:** 4KB Paging, Bump Frame Allocator (Simulated).

## 2. Memory Map (Physical)
The QEMU `virt` board maps memory and MMIO as follows:

| Start Address | End Address | Description | Access |
| :--- | :--- | :--- | :--- |
| `0x1000_0000` | `0x1000_1000` | UART0 (Serial Console) | RW (MMIO) |
| `0x1000_1000` | `0x1000_2000` | VirtIO Block Device | RW (MMIO) |
| `0x8000_0000` | `0x8040_0000` | Kernel Code/Data (~4MB) | RWX |
| `0x8040_0000` | `0x8800_0000` | Dynamic Heap / User Pages | RW (Free Memory) |

## 3. Virtual Address Space (Sv32)
Each process has its own page table. The kernel (top 1GB) is mapped identically in every process to facilitate traps.

| Virtual Address | Maps To | Description |
| :--- | :--- | :--- |
| `0x0000_0000` | User Code/Data | User Space (ELF load address varies) |
| `0xF000_0000` | Physical Frame | User Stack (Grows Down) |
| `0x8000_0000` | `0x8000_0000` | Kernel Identity Map (Global) |

## 4. Trap Handling Model
Ferrous uses a "trampoline" or direct-map approach depending on the assignment stage.

1.  **Exception/Interrupt:** CPU jumps to `stvec` (Supervisor Trap Vector).
2.  **Context Save:** The `TrapFrame` struct is pushed onto the **Kernel Stack** of the current thread.
3.  **Dispatch:** Rust code (`trap::handler`) identifies the cause (Syscall, Timer, Page Fault).
4.  **Handling:**
    *   **Syscall:** Dispatched to `syscalls.rs`.
    *   **Timer:** Calls `scheduler.tick()`.
5.  **Restore:** `sret` instruction restores context and returns to User Mode.

## 5. The Thread Control Block (TCB)
Located in `crates/ferrous-kernel/src/thread/tcb.rs`.
```rust
pub struct Thread {
    pub id: ThreadId,
    pub context: Context,    // Saved registers (ra, sp, etc.)
    pub state: ThreadState,  // Ready, Running, Blocked
    pub stack_top: u32,      // Kernel Stack Pointer
    pub process_id: u32,     // Owner Process
}
```

## 6. System Call Interface (ABI)
Parameters are passed in registers `a0` through `a5`. The return value is placed in `a0`.
*   `ecall` triggers the trap.
*   **System Call Numbers:** Defined in `syscall.rs`.
    *   1: `Sleep`
    *   2: `Yield`
    *   10: `Fork`
    *   11: `Exec`
    *   ...
