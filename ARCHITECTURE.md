# Ferrous - Educational Operating System Architecture Specification

**Version**: 1.1
**Date**: February 13, 2026
**Status**: Reference Implementation

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [System Overview](#system-overview)
3. [Design Principles](#design-principles)
4. [Architecture Layers](#architecture-layers)
5. [Crate Organization](#crate-organization)
6. [Build System (xtask)](#build-system-xtask)
7. [Component Specifications](#component-specifications)
8. [Assignment Structure](#assignment-structure)

---

## Executive Summary

**Ferrous** is a modern educational operating system framework built in Rust, designed to teach core OS concepts through hands-on implementation. It provides a RISC-V (RV32IMA) simulator and a partially-implemented kernel that students complete through structured assignments.

### Key Characteristics

- **Language**: Rust (idiomatic, type-safe, maintainable)
- **Target Architecture**: RISC-V RV32IMA subset
- **Execution Model**: Deterministic interpretation
- **Development Strategy**: "Strip and fill" - Students receive a working skeleton and fill in the missing pieces.
- **Educational Focus**: Clarity and correctness over performance
- **License**: MIT

---

## System Overview

### High-Level Architecture

```text
┌──────────────────────────────────────────────────────────┐
│                    Student Programs                      │
│               (Rust no_std → RISC-V ELF)                 │
│                 Executes on Simulated CPU                │
└──────────────────────────────────────────────────────────┘
                          ↓ syscalls (ecall)
┌──────────────────────────────────────────────────────────┐
│              RISC-V Simulator (ferrous-vm)               │
│     • CPU: RV32IMA interpreter                           │
│     • Memory: Physical memory + MMU                      │
│     • Devices: Console, Disk, Timer, Network             │
└──────────────────────────────────────────────────────────┘
                          ↓ traps (Host Calls)
┌──────────────────────────────────────────────────────────┐
│                    Ferrous Kernel                        │
│             (Student Implements in Host Rust)            │
│  ┌──────────┬──────────┬─────────┬──────────┬─────────┐  │
│  │ Threads  │  Sync    │ Virtual │   File   │ Network │  │
│  │Scheduler │Primitives│ Memory  │  System  │  Stack  │  │
│  └──────────┴──────────┴─────────┴──────────┴─────────┘  │
│          Executes natively on Host CPU                   │
└──────────────────────────────────────────────────────────┘
```

---

## Design Principles

### 1. Type Safety First
- Use newtypes for all domain concepts (ThreadHandle, PhysAddr, etc.)
- `Result<T, E>` for all fallible operations
- No unwrap() in production code paths

### 2. Clear Boundaries
- Trait-based abstractions between layers
- Explicit dependencies (never circular)
- Each crate has a single, well-defined responsibility

### 3. Idiomatic Rust
- Follow Rust API guidelines
- Use std patterns (Iterator, From/Into, Display, etc.)

---

## Architecture Layers

### Layer 1: Virtual Machine (ferrous-vm)
**Responsibility**: Simulate RISC-V hardware
**Interface**: Trait-based (TrapHandler, Device)

### Layer 2: Kernel (ferrous-kernel)
**Responsibility**: Operating system implementation. This is where students do most of their work.
**Components**:
- Thread scheduler
- Synchronization primitives
- Virtual memory manager
- File system
- Network stack

### Layer 3: User Library (ferrous-user)
**Responsibility**: Safe API for user programs (no_std).
**Interface**: Rust-style functions wrappers around syscalls.

### Layer 4: Build System (xtask)
**Responsibility**: Cross-platform build automation, testing, and assignment distribution.

---

## Crate Organization

### Workspace Structure

```text
ferrous/
├── Cargo.toml                    # Workspace root
├── .cargo/
│   └── config.toml               # Aliases for xtask
├── crates/
│   ├── ferrous-vm/               # RISC-V simulator
│   ├── ferrous-kernel/           # OS kernel (Student Workspace)
│   ├── ferrous-user/             # User program library
│   └── xtask/                    # Build system
├── examples/                     # Example user programs
├── assignments/                  # Generated assignment repositories
└── tests/                        # Integration tests
```

---

## Build System (xtask)

We use `cargo-xtask` to manage build processes in a cross-platform way, replacing Makefiles or shell scripts.

### Common Commands

- `cargo x build-host`: Builds the kernel and VM.
- `cargo x build-user`: Builds the user-space example programs (cross-compiles to RISC-V).
- `cargo x fs`: Manages the filesystem image (creates `disk.img`).
- `cargo x run-shell`: Builds everything and launches the interactive OS shell.

---

## Component Specifications

(Refer to source code docs for detailed API specifications)

### Kernel Modules

The kernel is modularized into directories:
- `thread/`: Scheduling and TCBs.
- `sync/`: Semaphores, Mutexes, CondVars.
- `process/`: Process lifecycle and ELF loading.
- `fs/`: Virtual File System.
- `memory.rs`: Memory management.
- `syscall.rs`: Main syscall dispatcher.

---

## Assignment Structure

Assignments are distributed as stripped-down versions of the reference implementation.

### The "Strip and Fill" Model

1. **Reference Implementation**: The full working OS is maintained in `crates/ferrous-kernel`.
2. **Assignment Markers**: Code to be removed is marked with `// TODO: Assignment X`.
3. **Extraction**: The build system extracts specific assignments by removing the marked implementation code, leaving function signatures and documentation.

### Student Experience

Students receive a repo that compiles (usually) but has unimplemented functions. They must implement the logic to pass the provided tests.

Example of a stripped function:

```rust
pub fn schedule(&mut self) -> Option<ThreadHandle> {
    // TODO: Assignment 1
    // Implement round-robin scheduling here.
    // 1. Check if current thread is still running
    // 2. Rotate the ready queue
    // 3. Return the next thread
    todo!("Implement schedule")
}
```
