# Ferrous Operating System Course
> *CS 530: Advanced Operating Systems Implementation*

Welcome to **Ferrous**, a modern, educational operating system framework designed to teach the internal mechanics of operating systems. Unlike traditional OS courses that use C, Ferrous is built entirely in **Rust**, leveraging its strong type system to prevent common classes of bugs (like use-after-free and data races) while retaining the low-level control required for kernel development.

This repository contains the **Reference Implementation** of the Ferrous Kernel. As a student, you will be implementing core subsystems of this kernel across a series of structured assignments.

## 🎯 Learning Objectives

By the end of this course, you will have implemented a fully functional Unix-like operating system kernel, capable of:
*   **Multithreading:** Implementing cooperative and preemptive thread scheduling.
*   **Synchronization:** Building semaphores, mutexes, and condition variables from scratch.
*   **Virtual Memory:** Managing page tables, address translation, and demand paging.
*   **File Systems:** Designing an inode-based file system with directory structures.
*   **Process Management:** Implementing `fork`, `exec`, and `wait` semantics.

## 🏗️ System Architecture

Ferrous runs on a custom **RISC-V Virtual Machine** (`ferrous-vm`) included in this repository. This simplifies development by allowing you to run your kernel as a standard user-space process on your host machine (Linux, macOS, or Windows), while still simulating a realistic hardware environment.

```text
┌──────────────────────────────────┐
│      User Shell & Utilities      │  <-- Compiled to RISC-V (Guest)
│    (examples/shell, /bin/ls)     │
└────────────────┬─────────────────┘
                 │ System Calls (ecall)
┌────────────────▼─────────────────┐
│         Ferrous Kernel           │  <-- YOU WILL IMPLEMENT THIS
│   (Threading, FS, Syscalls...)   │
└────────────────┬─────────────────┘
                 │ Traps / Host Calls
┌────────────────▼─────────────────┐
│   Ferrous Virtual Machine (VM)   │  <-- Simulates CPU, RAM, Disk
│       (RV32IMA Interpreter)      │
└──────────────────────────────────┘
```

## 🚀 Getting Started

### Prerequisites
*   **Rust Toolchain:** Install Rust via [rustup](https://rustup.rs/).
*   **RISC-V Target:** You need the cross-compilation target for building user programs:
    ```bash
    rustup target add riscv32i-unknown-none-elf
    ```
*   **Git:** For version control.

### Building the System

We use a custom build system written in Rust called `xtask`. This abstracts away the complexity of cross-compiling the guest programs and creating disk images.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/ferrous-os/ferrous.git
    cd ferrous
    ```

2.  **Build the Host Tools (VM & Kernel):**
    ```bash
    cargo x build-host
    ```

3.  **Build the Guest Programs (Shell, Hello World):**
    ```bash
    cargo x build-user
    ```

4.  **Run the Smoke Test:**
    This command compiles a simple "Hello World" program and runs it inside the VM, bypassing the file system. It verifies that your toolchain is correctly set up.
    ```bash
    cargo x run-hello
    ```
    *Expected Output:* `Hello from Ferrous!`

### Running the Full OS

To run the full operating system with a persistent file system and interactive shell:

```bash
cargo x run-shell
```

This command performs the following steps automatically:
1.  Compiles all user programs (shell, echo, cat, etc.) for RISC-V.
2.  Creates a disk image (`disk.img`) and formats it with FerrousFS.
3.  Mounts the disk image.
4.  Boots the kernel and launches the shell.

## 📚 Repository Structure

The project is organized as a Cargo Workspace with multiple crates:

*   **`crates/ferrous-kernel`**: The core OS kernel. **This is where you will do 90% of your work.**
    *   `src/thread/`: Scheduler and context switching.
    *   `src/sync/`: Synchronization primitives.
    *   `src/fs/`: File system logic.
    *   `src/process/`: Process lifecycle management.
*   **`crates/ferrous-vm`**: The RISC-V simulator. You generally do not need to modify this unless you are debugging a hardware issue.
*   **`crates/ferrous-user`**: The system call library used by user programs (similar to `libc`).
*   **`crates/xtask`**: The build automation system.
*   **`examples/`**: Source code for user programs (shell, etc.).

## 📝 Assignment Roadmap

The course is divided into 6 assignments. Each assignment builds upon the previous one.

| Assignment | Topic | Description |
| :--- | :--- | :--- |
| **A1** | **Threads** | Implement `ThreadControlBlock`, context switching, and a Round-Robin scheduler. |
| **A2** | **Synchronization** | Implement semaphores and mutexes to protect kernel data structures. |
| **A3** | **Memory** | Implement the `sbrk` system call and manage kernel heap allocations. |
| **A4** | **Processes** | Implement `fork`, `exec`, and `waitpid` to support process isolation. |
| **A5** | **File Systems** | Implement inode traversal (`read_inode`) and file data reading (`read_data`). |
| **A6** | **Networking** | (Advanced) Implement a basic network stack and socket API. |

### How to Work on Assignments

1.  Navigate to the relevant file in `crates/ferrous-kernel`.
2.  Look for `// TODO: Assignment X` markers.
3.  Read the documentation comments (`///`) carefully to understand the contract of the function you are implementing.
4.  Implement the logic.
5.  Run the tests (instructions provided in each assignment handout).

## 🧪 Testing

We separate tests into **Host Tests** (running natively on your machine) and **Guest Tests** (running inside the VM).

*   **Run Kernel Unit Tests:**
    ```bash
    cargo test -p ferrous-kernel
    ```

*   **Run Integration Tests:**
    Specific `cargo x` commands will be provided for each assignment to run relevant integration tests.

## ⚠️ Academic Integrity

This repository contains the **Reference Implementation**. In a real course setting, the solutions in `ferrous-kernel` would be stripped out and provided as skeleton code.

If you are a student taking this course: **Do not view the history or reference solution branches** if instructed by your professor. The goal is for you to learn by implementing these systems yourself.

---
*Maintained by the Ferrous OS Team for educational use.*
