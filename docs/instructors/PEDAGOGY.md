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

## 4. Academic Integrity & Anti-Plagiarism
Because Ferrous is a novel course platform, solutions are not yet widely available online. However, the modular nature of the assignments allows for targeted modification:
*   **Variable Parameters:** Instructors can easily modify `TIME_SLICE` durations, stack sizes, or scheduling policies to render previous solutions obsolete.
*   **MOSS Integration:** Rust code is highly amenable to MOSS (Measure of Software Similarity) analysis due to its strict formatting (rustfmt) and structure.

## 5. Prerequisite Knowledge
Students are expected to possess:
*   **Computer Architecture:** Familiarity with registers, stacks, interrupts, and basic assembly (RISC-V or x86).
*   **Systems Programming:** Experience with pointers, memory allocation, and debugging.
*   **Rust Proficiency:** While not strictly required, a "crash course" week (Week 0) is recommended for students new to the borrow checker.
