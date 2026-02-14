# Getting Started with Ferrous OS

## 1. Prerequisites
Ferrous OS development requires a Linux or macOS environment (or WSL2 on Windows). You cannot develop natively on Windows CMD/PowerShell due to build script dependencies.

### Required Tools
*   **Rustup:** The Rust toolchain installer.
*   **QEMU:** A machine emulator and virtualizer.
*   **RISC-V Toolchain:** Linkers and debuggers for the target architecture.

## 2. Quick Setup (Ubuntu/WSL2)
Run the following commands to set up your environment:

```bash
# 1. Install System Dependencies
sudo apt update
sudo apt install build-essential qemu-system-misc gdb-multiarch

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. Configure Rust Toolchain for Ferrous
# We use Nightly Rust for experimental features like inline assembly.
rustup override set nightly
rustup target add riscv32imac-unknown-none-elf
rustup component add llvm-tools-preview
cargo install cargo-binutils
```

## 3. Building and Running
Navigate to an assignment directory (e.g., `assignments/assignment-1-threads`).

### Compile Only
```bash
cargo build
```

### Run in QEMU
```bash
cargo run
```
You should see output similar to:
```text
[ferrous-boot] Hello, World!
[kernel] Initializing Thread Manager...
[kernel] Scheduler: Round Robin
...
```
To exit QEMU, press `Ctrl+A` then `X`.

### Run Tests
```bash
cargo test
```
This will automatically launch QEMU, run the kernel unit tests, and print the results.

## 4. Project Structure
Each assignment is a **Workspace** containing multiple crates:

*   `crates/ferrous-kernel`: **The Core OS.** You will do 90% of your work here.
    *   `src/thread/`: Scheduler logic (Assignment 1).
    *   `src/sync/`: Locks and semaphores (Assignment 2).
    *   `src/memory.rs`: Page table management (Assignment 3).
    *   `src/process/`: Process loading (Assignment 4).
    *   `src/fs/`: File system (Assignment 5).
*   `crates/ferrous-vm`: Low-level hardware definitions (Registers, Page Tables). **Read-only**.
*   `crates/ferrous-user`: User-space library (like `libc`).
*   `examples/`: Sample user programs (Shell, Echo, etc.).

## 5. Tips for Success
1.  **Read the Docs:** `docs/students/ARCHITECTURE.md` contains the memory map you *will* need for Assignment 3.
2.  **Use `grep`:** The codebase is large. If you see a function `context_switch`, grep for where it is defined.
3.  **Ask for Help:** OS debugging is hard. If QEMU hangs without output, check your stack pointers!
