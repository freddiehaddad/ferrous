# Getting Started with Ferrous OS

## 1. Prerequisites
Ferrous OS development requires a specific set of tools to compile and run the operating system. We support **macOS**, **Windows**, and **Linux**.

### Core Requirements
*   **Rust:** The specific "Nightly" toolchain is required for bare-metal features.
*   **RISC-V Target:** The cross-compilation target `riscv32i-unknown-none-elf`.
*   **QEMU (Optional):** While Ferrous runs on a custom simulator (`ferrous-vm`), QEMU is highly recommended for advanced debugging and portability testing.

## 2. Environment Setup

### 🍎 macOS (Apple Silicon & Intel)
We recommend using **Homebrew** for package management.

1.  **Install Rust:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"
    ```
2.  **Add RISC-V Targets:**
    ```bash
    rustup override set nightly
    rustup target add riscv32i-unknown-none-elf
    rustup component add llvm-tools-preview
    cargo install cargo-binutils
    ```
3.  **Install QEMU (Optional):**
    ```bash
    brew tap riscv-software-src/riscv
    brew install qemu
    ```
    *Note for M1/M2 Users:* Ferrous runs natively via `ferrous-vm`, so you will not experience performance penalties from architecture mismatch.

### 🪟 Windows
You have two robust options. **Native** is simpler for just running the assignments. **WSL2** is recommended if you are already comfortable with Linux or plan to use GDB heavily.

#### Option A: Windows Native (Recommended for Simplicity)
1.  **Install Rust:** Download `rustup-init.exe` from [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  **Toolchain Setup:** Open PowerShell or Command Prompt:
    ```powershell
    rustup override set nightly
    rustup target add riscv32i-unknown-none-elf
    rustup component add llvm-tools-preview
    cargo install cargo-binutils
    ```
3.  **Git Configuration (Crucial):**
    When installing Git for Windows, ensure you select **"Checkout as-is, commit Unix-style line endings"**. The build scripts rely on `\n` line endings.

#### Option B: WSL2 (Ubuntu 24.04 LTS)
1.  Open PowerShell as Administrator and run: `wsl --install`
2.  Reboot if prompted.
3.  Open "Ubuntu" from the Start Menu.
4.  Follow the **Linux** instructions below inside the Ubuntu terminal.
5.  **Important:** Clone your repo inside the Linux filesystem (`~/projects/ferrous`), **NOT** on the Windows C: drive (`/mnt/c/...`). Compiling on the Windows filesystem from WSL2 is extremely slow.

### 🐧 Linux (Ubuntu/Debian)
The gold standard for OS development.

1.  **Install Build Tools:**
    ```bash
    sudo apt update
    sudo apt install build-essential curl git qemu-system-misc gdb-multiarch
    ```
2.  **Install Rust:**
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"
    ```
3.  **Configure Toolchain:**
    ```bash
    rustup override set nightly
    rustup target add riscv32i-unknown-none-elf
    rustup component add llvm-tools-preview
    cargo install cargo-binutils
    ```

## 3. Building and Running
Navigate to an assignment directory (e.g., `assignments/assignment-1-threads`).

### Compile Only
```bash
cargo build
```

### Run in Simulator
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
To exit, press `Ctrl+C`.

### Run Tests
```bash
cargo test
```
This will automatically launch the kernel in "Test Mode", run the unit tests, and print the results.

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
3.  **Ask for Help:** OS debugging is hard. If the system hangs without output, check your stack pointers!
