# Ferrous OS: Pedagogical Guide & Instructor Manual

## 1. Course Philosophy
Ferrous OS is designed for a graduate-level course in Operating Systems Engineering. Unlike traditional courses based on C (xv6, Minix) or Java (Nachos), Ferrous leverages **Rust** to bridge the gap between low-level hardware control and modern software engineering principles.

The primary pedagogical objective is to move students beyond *observing* OS concepts to *implementing* robust kernel abstractions. The use of Rust forces students to explicitly reason about:
*   **Memory Safety:** The ownership model makes memory leaks and race conditions compile-time errors rather than runtime Heisenbugs.
*   **Concurrency:** Rust's `Send` and `Sync` traits provide a semantic framework for teaching thread safety that is strictly enforced by the compiler.
*   **Unsafe Contracts:** Students learn to distinguish between safe abstractions and the "unsafe" hardware boundary, a critical skill for systems programmers.

## 2. Learning Objectives (Bloom's Taxonomy)
Upon completion of this course, students will be able to:
1.  **Synthesize** a preemptive thread scheduler (Round Robin) and justify its performance characteristics vs. cooperative scheduling.
2.  **Construct** synchronization primitives (Semaphores, Condition Variables) using atomic hardware instructions.
3.  **Design** a Virtual Memory subsystem that isolates kernel and user address spaces using hardware Page Tables (RISC-V Sv32).
4.  **Implement** a process lifecycle manager capable of loading ELF binaries, handling system calls (Fork, Exec), and managing process termination.
5.  **Architect** a persistent file system (Inode-based) that abstracts physical storage blocks into logical file streams.

## 3. Assignment Roadmap
The course is structured into five cumulative assignments.

| Unit | Topic | Core Concepts | Rust Concepts |
| :--- | :--- | :--- | :--- |
| **A1** | **Threading** | Context Switching, Scheduling, TCBs | `Box`, `VecDeque`, Raw Pointers (Assembly) |
| **A2** | **Synchronization** | Atomics, Spinlocks, Semaphores, Monitors | `UnsafeCell`, Interior Mutability, `AtomicUsize` |
| **A3** | **Virtual Memory** | Paging, Address Translation, TLB, Page Faults | `BitFlags`, Page Table Entry (PTE) encoding |
| **A4** | **Processes** | Syscall Interface, User/Kernel Boundary, ELFs | Traps, CSRs (SATP, SEPC), Privilege Modes |
| **A5** | **File Systems** | Inodes, Directory Entries, Block Caching | Serialization, Buffer Management |

### Assignment Details

#### **Assignment 1: Threads & Scheduling**
*   **Goal:** Implement a cooperative and preemptive multitasking system.
*   **Key Files:**
    *   `crates/ferrous-kernel/src/thread/tcb.rs`: `ThreadControlBlock`
    *   `crates/ferrous-kernel/src/thread/mod.rs`: `create_thread`, `yield_thread`
*   **Implementation Steps:**
    1.  **Context Switching:** Implement `save_context` and `restore_context` (often inline assembly or carefully managed struct fields).
    2.  **TCB:** Define the `ThreadControlBlock` to store registers (PC, SP, S0-S11, etc.).
    3.  **Scheduler:** Implement a Round-Robin queue in `scheduler.rs`.
*   **Testing Strategy:**
    *   `cargo x run-test threads`: Spawns multiple threads printing different characters. Verify they interleave and don't crash.

#### **Assignment 2: Synchronization**
*   **Goal:** Prevent data races in your scheduler and kernel data structures.
*   **Key Files:**
    *   `crates/ferrous-kernel/src/sync/mod.rs`
    *   `crates/ferrous-kernel/src/thread/mod.rs` (`block_current_thread`, `wake_thread`)
*   **Implementation Steps:**
    1.  **Semaphore:** Implement `down()` (decrement/sleep) and `up()` (increment/wake).
    2.  **Mutex:** Build on top of semaphores (binary semaphore).
    3.  **Condition Variables:** Implement `wait()` and `notify()`.
*   **Testing Strategy:**
    *   `cargo x run-test sync`: A producer-consumer problem. If implemented incorrectly, the system will deadlock or print garbage.

#### **Assignment 3: Virtual Memory (Heap)**
*   **Goal:** Allow the kernel and users to dynamically allocate memory (`malloc`).
*   **Key Files:**
    *   `crates/ferrous-kernel/src/memory.rs`
    *   `crates/ferrous-vm` (check `mmu.rs` to understand the page table format, though you likely won't edit it).
*   **Implementation Steps:**
    1.  **sbrk Syscall:** Implement the handler for `SYS_SBRK`.
    2.  **Program Break:** Track the current end of the heap for the current process.
    3.  **Page Mapping:** When `sbrk` grows, you must ensure new pages are mapped in the page table (software-managed TLB/PT logic).
*   **Testing Strategy:**
    *   `cargo x run-test sbrk`: Allocates a large array, writes to it, reads back. Verifies no Page Faults occur.

#### **Assignment 4: Processes**
*   **Goal:** Implement process isolation and lifecycle management.
*   **Key Files:**
    *   `crates/ferrous-kernel/src/process/mod.rs`
    *   `crates/ferrous-kernel/src/process/syscalls.rs`
*   **Implementation Steps:**
    1.  **Fork:** Deep copy the current thread's memory space and TCB. Return `0` to child, `child_pid` to parent.
    2.  **Exec:** Replace the current memory space with a new binary loaded from disk.
    3.  **Wait:** Block until a child process enters the `Terminated` state.
*   **Testing Strategy:**
    *   `cargo x run-shell`: The shell itself relies on `fork` and `exec` to run commands. If the shell works, you win.

#### **Assignment 5: File Systems**
*   **Goal:** Read files from the `disk.img` provided by the build system.
*   **Key Files:**
    *   `crates/ferrous-kernel/src/fs/mod.rs`
    *   `crates/ferrous-kernel/src/fs/block.rs`
*   **Implementation Steps:**
    1.  **Inode Reading:** Implement `read_inode(inode_idx)` to fetch file metadata from the inode table.
    2.  **Directory Traversal:** Implement `find_inode(path)` to resolve `/bin/ls` to an inode number.
    3.  **Data Reading:** Implement `read_data(inode, offset, buffer)` to traverse direct and indirect block pointers.
*   **Testing Strategy:**
    *   `cargo x fs`: Creates a `hello.txt`.
    *   `cat hello.txt` inside the shell. If you see the text, your FS driver is working.

## 4. Academic Integrity & Anti-Plagiarism
Because Ferrous is a novel course platform, solutions are not yet widely available online. However, the modular nature of the assignments allows for targeted modification:
*   **Variable Parameters:** Instructors can easily modify `TIME_SLICE` durations, stack sizes, or scheduling policies to render previous solutions obsolete.
*   **MOSS Integration:** Rust code is highly amenable to MOSS (Measure of Software Similarity) analysis due to its strict formatting (rustfmt) and structure.

## 5. Prerequisite Knowledge
Students are expected to possess:
*   **Computer Architecture:** Familiarity with registers, stacks, interrupts, and basic assembly (RISC-V or x86).
*   **Systems Programming:** Experience with pointers, memory allocation, and debugging.
*   **Rust Proficiency:** While not strictly required, a "crash course" week (Week 0) is recommended for students new to the borrow checker.
