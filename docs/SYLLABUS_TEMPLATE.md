# Course Syllabus: Operating Systems Engineering
**Course Code:** CS 550 | **Term:** Fall 202X

## Course Description
This course provides an in-depth examination of the design and implementation of modern operating systems. Unlike introductory courses that focus on *using* OS APIs, this course focuses on *building* them. Students will implement a functional Unix-like kernel in Rust, covering scheduling, concurrency, virtual memory, processes, and file systems.

## Learning Outcomes
By the end of this course, students will be able to:
*   Understand the hardware/software interface (RISC-V architecture).
*   Implement safe abstractions over unsafe hardware primitives using Rust.
*   Debug complex concurrent systems and race conditions.
*   Analyze trade-offs in scheduling algorithms and memory management strategies.

## Prerequisites
*   **CS 300 (Data Structures):** Proficiency with queues, trees, and hash maps.
*   **CS 400 (Computer Architecture):** Understanding of registers, stacks, and interrupts.
*   **Programming:** Strong experience in C, C++, or Rust.

## Technical Stack
*   **Language:** Rust (Nightly)
*   **Target:** RISC-V (32-bit)
*   **Emulator:** QEMU
*   **Platform:** Linux / WSL2

## Schedule & Roadmap

| Week | Topic | Assignment | Due Date |
| :--- | :--- | :--- | :--- |
| 1 | Intro to Rust & Systems Programming | A0: Warmup | Week 2 |
| 2-3 | Context Switching & Scheduling | **A1: Threads** | Week 4 |
| 4-5 | Concurrency & Synchronization | **A2: Sync** | Week 6 |
| 6-7 | Virtual Memory & Paging | **A3: VM** | Week 8 |
| 8 | *Midterm Exam* | - | - |
| 9-10 | Processes & System Calls | **A4: Processes** | Week 11 |
| 11-12 | File Systems & Persistence | **A5: File Systems** | Week 13 |
| 13-14 | Advanced Topics (Networking/Security) | Final Project | Week 15 |

## Grading Policy
*   **Assignments (5):** 60%
    *   Each assignment includes automated tests (80%) and design review (20%).
*   **Midterm Exam:** 20%
*   **Final Project:** 20%

## Academic Integrity
Code submitted must be your own. You may discuss high-level algorithms with classmates, but sharing code snippets is strictly prohibited. All submissions are subject to MOSS (Measure of Software Similarity) analysis.

## Resources
*   **The Rust Programming Language (The Book):** [doc.rust-lang.org/book](https://doc.rust-lang.org/book/)
*   **RISC-V Reader:** [riscv.org/technical/specifications](https://riscv.org/technical/specifications/)
*   **Ferrous OS Docs:** See `docs/` in the repository.
