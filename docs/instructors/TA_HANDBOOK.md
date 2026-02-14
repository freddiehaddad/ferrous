# Teaching Assistant Handbook

## 1. Environment Setup
Ferrous requires a specific toolchain to cross-compile for RISC-V. Ensure all lab machines and grading servers have the following:

```bash
# Rust Nightly (Required for naked_functions, inline_asm)
rustup override set nightly
rustup target add riscv32imac-unknown-none-elf
cargo install cargo-binutils
rustup component add llvm-tools-preview

# QEMU (RISC-V 32-bit)
# Ubuntu: apt install qemu-system-misc
```

## 2. Grading Workflow
Each assignment folder acts as a standalone cargo workspace.

### Automated Testing
Students are provided with a `ferrous-test` crate that runs unit tests inside the kernel.
To grade an assignment:
1.  Navigate to the assignment directory (e.g., `assignments/assignment-1-threads`).
2.  Run `cargo test`.
    *   This compiles the kernel in "Test Mode".
    *   It launches QEMU.
    *   It parses the UART output for "ok" or "FAILED".
    *   **Note:** If QEMU hangs, the test failed (deadlock or panic).

### Manual Inspection
Automated tests cannot catch subtle race conditions or poor design. Review critical files:
*   **A1:** `scheduler.rs` - Check for O(N) operations in critical paths.
*   **A2:** `sync/` - Ensure interrupts are disabled when acquiring spinlocks to prevent deadlock.
*   **A3:** `memory.rs` - Verify that kernel pages are NOT mapped with `User` permission (security vulnerability).

## 3. Common Student Pitfalls

### "The Borrow Checker hates me!"
*   **Symptom:** Students try to implement a linked list or self-referential struct for the scheduler.
*   **Solution:** Guide them toward using `VecDeque` with `ThreadHandle` (IDs) instead of references. Or use `Arc<Mutex<T>>` if absolutely necessary, though the kernel typically uses raw pointers wrapped in `unsafe` blocks for core structures.

### Double Faults / Triple Faults
*   **Symptom:** QEMU keeps resetting (boot loop).
*   **Cause:** Often a stack overflow (kernel stack is small, ~16KB) or an unhandled exception during the trap handler.
*   **Debug:** Run `cargo run --release` to see if optimizations reduce stack usage, or attach GDB (see Section 4).

### "It works sometimes" (Race Conditions)
*   **Symptom:** Tests pass 9/10 times.
*   **Cause:** Improper interrupt masking.
*   **Check:** specific critical sections in `syscalls.rs`. If they modify global state without acquiring a lock or disabling interrupts, it's a bug.

## 4. Debugging with GDB
Ferrous supports remote debugging via QEMU.

1.  **Terminal 1 (Run OS):**
    ```bash
    qemu-system-riscv32 -machine virt -bios default -kernel target/.../ferrous-kernel -s -S
    ```
    (`-s` opens port 1234, `-S` freezes CPU at startup)

2.  **Terminal 2 (GDB):**
    ```bash
    riscv64-unknown-elf-gdb target/.../ferrous-kernel
    (gdb) target remote :1234
    (gdb) break bootstrap_process
    (gdb) continue
    ```
