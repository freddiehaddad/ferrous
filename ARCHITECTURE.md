# Ferrous - Educational Operating System Architecture Specification

**Version**: 1.0  
**Date**: January 31, 2026  
**Status**: Design Specification

---

## Table of Contents

1. [Executive Summary](#executive-summary)
1. [System Overview](#system-overview)
1. [Design Principles](#design-principles)
1. [Architecture Layers](#architecture-layers)
1. [Crate Organization](#crate-organization)
1. [Core Type System](#core-type-system)
1. [Component Specifications](#component-specifications)
1. [Implementation Roadmap](#implementation-roadmap)
1. [Testing Strategy](#testing-strategy)
1. [Assignment Structure](#assignment-structure)

---

## Executive Summary

**Ferrous** is a modern educational operating system framework built in Rust, designed to teach core OS concepts through hands-on implementation. It provides a RISC-V (RV32IMA) simulator and a partially-implemented kernel that students complete through structured assignments.

### Key Characteristics

- **Language**: Rust (idiomatic, type-safe, maintainable)
- **Target Architecture**: RISC-V RV32IMA subset
- **Execution Model**: Deterministic interpretation
- **Development Strategy**: Iterative vertical slices
- **Educational Focus**: Clarity and correctness over performance
- **License**: MIT

### Success Criteria

1. Professional-grade, maintainable codebase
1. Type-safe, idiomatic Rust throughout
1. Clear separation of concerns
1. Comprehensive test coverage
1. Excellent documentation
1. Student-friendly APIs

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
- Leverage Rust's type system to prevent bugs at compile time

### 2. Clear Boundaries

- Trait-based abstractions between layers
- Explicit dependencies (never circular)
- Each crate has a single, well-defined responsibility
- Public APIs are minimal and carefully considered

### 3. Idiomatic Rust

- Follow Rust API guidelines
- Use std patterns (Iterator, From/Into, Display, etc.)
- Leverage derive macros where appropriate
- Prefer composition over inheritance

### 4. Educational Clarity

- Code should be readable and instructive
- Comments explain "why", not "what"
- Examples demonstrate correct usage
- Error messages guide students

### 5. Testability

- All components are independently testable
- Mock implementations for external dependencies
- Property-based testing for critical algorithms
- Integration tests validate end-to-end behavior

### 6. Maintainability

- No clever code - prefer obvious over concise
- Consistent naming and patterns throughout
- Refactor-friendly architecture
- Future-proof design decisions

---

## Architecture Layers

### Layer 1: Virtual Machine (ferrous-vm)

**Responsibility**: Simulate RISC-V hardware

**Key Components**:
- CPU interpreter (RV32IMA)
- Physical memory subsystem
- Memory-mapped devices
- Trap/interrupt handling
- Privilege mode enforcement

**Dependencies**: None (except std library)

**Interface**: Trait-based (TrapHandler, Device)

---

### Layer 2: Kernel (ferrous-kernel)

**Responsibility**: Operating system implementation

**Key Components**:
- Thread scheduler
- Synchronization primitives
- Virtual memory manager
- File system
- Network stack
- System call handlers

**Dependencies**: ferrous-vm (traits only), alloc

**Interface**: System call ABI

---

### Layer 3: User Library (ferrous-user)

**Responsibility**: Safe API for user programs

**Key Components**:
- System call wrappers
- Thread management API
- I/O operations
- Network sockets

**Dependencies**: None (no std)

**Interface**: Rust-style functions

---

### Layer 4: Runtime & Tools (ferrous-runtime, ferrous-cli)

**Responsibility**: Development and testing infrastructure

**Key Components**:
- ELF loader
- Test runner
- Debugger
- CLI interface

**Dependencies**: ferrous-vm, ferrous-kernel

**Interface**: CLI and programmatic APIs

---

## Crate Organization


### Workspace Structure

```text
ferrous/
├── Cargo.toml                    # Workspace root
├── README.md
├── LICENSE
├── ARCHITECTURE.md               # This document
├── .gitignore
│
├── crates/
│   ├── ferrous-vm/              # RISC-V simulator
│   ├── ferrous-kernel/          # OS kernel
│   ├── ferrous-user/            # User program library
│   ├── ferrous-runtime/         # Runtime support
│   ├── ferrous-cli/             # Command-line interface
│   └── ferrous-test/            # Testing framework
│
├── examples/                     # Example user programs
│   ├── hello-world/
│   ├── threads/
│   └── ...
│
├── assignments/                  # Student assignment templates
│   ├── assignment-1-threads/
│   └── ...
│
├── solutions/                    # Reference solutions (in-repo, separate dir)
│   ├── assignment-1-threads/
│   └── ...
│
├── tests/                       # Integration tests
│   ├── vm_tests.rs
│   ├── kernel_tests.rs
│   └── ...
│
└── docs/                        # mdBook documentation
    ├── book.toml
    └── src/
        ├── SUMMARY.md
        ├── getting-started.md
        ├── assignments/
        └── reference/
```

### Crate Dependency Graph

```text
ferrous-cli
    ├── ferrous-runtime
    │   ├── ferrous-vm
    │   └── ferrous-kernel
    │       └── ferrous-vm (traits only)
    └── ferrous-test
        ├── ferrous-vm
        └── ferrous-kernel

ferrous-user
    └── (no dependencies, no std)

examples/*
    └── ferrous-user
```

**Key Principles**:
- No circular dependencies
- ferrous-vm has no dependencies (pure library)
- ferrous-kernel depends only on VM traits
- ferrous-user is completely standalone (no std)

---

## Core Type System

### Type Safety Strategy

Every domain concept gets its own type. Never use primitives (u32, usize) directly for domain concepts.

### Fundamental Types

#### Identity Types (Handle Pattern)

All use `NonZeroU32` for niche optimization:

```rust
// ferrous-kernel/src/types.rs

/// Thread identifier
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ThreadHandle(NonZeroU32);

/// Process identifier  
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ProcessHandle(NonZeroU32);

/// File descriptor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FileDescriptor(NonZeroU32);

/// Socket descriptor
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SocketHandle(NonZeroU32);

/// Semaphore handle
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SemaphoreHandle(NonZeroU32);

/// Lock handle
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LockHandle(NonZeroU32);

/// Condition variable handle
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CondVarHandle(NonZeroU32);
```

**Benefits**:
- Cannot mix up different ID types
- `Option<ThreadHandle>` is same size as `ThreadHandle` (niche optimization)
- Type-safe, self-documenting

#### Memory Address Types

```rust
// ferrous-vm/src/memory.rs

/// Physical memory address (cannot be dereferenced directly)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysAddr(u32);

/// Virtual memory address
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct VirtAddr(u32);

/// Page number (physical)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PhysPageNum(u32);

/// Page number (virtual)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct VirtPageNum(u32);
```

**Benefits**:
- Cannot accidentally use physical address as virtual (or vice versa)
- Compiler enforces correct address translation
- Clear intent in APIs

#### Time Types

```rust
// ferrous-vm/src/time.rs

use core::time::Duration;

/// Simulated time instant (monotonic)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct SimulatedInstant {
    ticks: u64,
}

impl SimulatedInstant {
    pub fn elapsed_since(&self, earlier: SimulatedInstant) -> Duration { ... }
    pub fn duration_since(&self, earlier: SimulatedInstant) -> Duration { ... }
}
```

**Benefits**:
- Cannot mix ticks with durations
- Type-safe time arithmetic
- Matches std::time patterns

#### Register Types

```rust
// ferrous-vm/src/cpu.rs

/// RISC-V register number (x0-x31)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Register(u8);

impl Register {
    pub const ZERO: Register = Register(0);  // x0
    pub const RA: Register = Register(1);    // x1 (return address)
    pub const SP: Register = Register(2);    // x2 (stack pointer)
    // ... all 32 registers as constants
    
    pub fn new(num: u8) -> Result<Self, InvalidRegister> {
        if num < 32 {
            Ok(Register(num))
        } else {
            Err(InvalidRegister(num))
        }
    }
}
```

**Benefits**:
- Cannot use invalid register numbers
- Self-documenting (Register::SP vs raw 2)
- Exhaustive matching on common registers

---

## Component Specifications

### Component 1: ferrous-vm (RISC-V Simulator)

#### Module Structure

```text
ferrous-vm/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── cpu.rs              # CPU state and execution
    ├── instruction.rs      # Instruction decoder
    ├── memory.rs           # Memory subsystem
    ├── mmu.rs              # Memory Management Unit (Sv32)
    ├── trap.rs             # Trap/interrupt handling
    ├── time.rs             # Time simulation
    ├── devices/
    │   ├── mod.rs
    │   ├── console.rs
    │   ├── disk.rs
    │   ├── timer.rs
    │   └── network.rs
    ├── error.rs            # Error types
    └── tests/
        └── instruction_tests.rs
```


#### Core Types

```rust
// lib.rs - Public API

pub struct VirtualMachine {
    cpu: Cpu,
    memory: Memory,
    devices: DeviceManager,
    trap_handler: Box<dyn TrapHandler>,
}

impl VirtualMachine {
    pub fn new(config: VmConfig, trap_handler: Box<dyn TrapHandler>) -> Result<Self, VmError>;
    pub fn load_program(&mut self, binary: &[u8], entry_point: VirtAddr) -> Result<(), VmError>;
    pub fn run(&mut self) -> Result<ExitReason, VmError>;
    pub fn step(&mut self) -> Result<StepResult, VmError>;  // Single instruction
    pub fn cpu(&self) -> &Cpu;
    pub fn memory(&self) -> &Memory;
}

pub struct VmConfig {
    pub memory_size: usize,          // Physical memory in bytes
    pub enable_mmu: bool,             // Enable virtual memory
    pub enable_timer: bool,
    pub timer_interval_ms: u64,
}

pub enum ExitReason {
    Halt,                  // Normal termination
    Breakpoint,            // Debugger breakpoint hit
    Error(VmError),        // Execution error
}

pub enum StepResult {
    Continue,              // Keep executing
    Trap(TrapCause),       // Trap occurred
    Exit(ExitReason),      // VM stopped
}
```

#### Instruction Representation

```rust
// instruction.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // RV32I Base
    Add { rd: Register, rs1: Register, rs2: Register },
    Addi { rd: Register, rs1: Register, imm: i32 },
    Sub { rd: Register, rs1: Register, rs2: Register },
    And { rd: Register, rs1: Register, rs2: Register },
    Or { rd: Register, rs1: Register, rs2: Register },
    Xor { rd: Register, rs1: Register, rs2: Register },
    Sll { rd: Register, rs1: Register, rs2: Register },
    Srl { rd: Register, rs1: Register, rs2: Register },
    Sra { rd: Register, rs1: Register, rs2: Register },
    
    // Loads/Stores
    Lw { rd: Register, rs1: Register, offset: i32 },
    Lb { rd: Register, rs1: Register, offset: i32 },
    Lh { rd: Register, rs1: Register, offset: i32 },
    Sw { rs1: Register, rs2: Register, offset: i32 },
    Sb { rs1: Register, rs2: Register, offset: i32 },
    Sh { rs1: Register, rs2: Register, offset: i32 },
    
    // Branches
    Beq { rs1: Register, rs2: Register, offset: i32 },
    Bne { rs1: Register, rs2: Register, offset: i32 },
    Blt { rs1: Register, rs2: Register, offset: i32 },
    Bge { rs1: Register, rs2: Register, offset: i32 },
    
    // Jumps
    Jal { rd: Register, offset: i32 },
    Jalr { rd: Register, rs1: Register, offset: i32 },
    
    // Upper immediate
    Lui { rd: Register, imm: u32 },
    Auipc { rd: Register, imm: u32 },
    
    // System
    Ecall,
    Ebreak,
    
    // RV32M Extension
    Mul { rd: Register, rs1: Register, rs2: Register },
    Div { rd: Register, rs1: Register, rs2: Register },
    Rem { rd: Register, rs1: Register, rs2: Register },
    
    // RV32A Extension (Atomic)
    LrW { rd: Register, rs1: Register },
    ScW { rd: Register, rs1: Register, rs2: Register },
    AmoswapW { rd: Register, rs1: Register, rs2: Register },
    AmoaddW { rd: Register, rs1: Register, rs2: Register },
}

impl Instruction {
    /// Decode a 32-bit instruction word
    pub fn decode(word: u32) -> Result<Self, DecodeError>;
}
```

#### Trap Handling Interface

```rust
// trap.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapCause {
    // Exceptions
    InstructionMisaligned { addr: VirtAddr },
    InstructionAccessFault { addr: VirtAddr },
    IllegalInstruction { instruction: u32 },
    Breakpoint,
    LoadAddressMisaligned { addr: VirtAddr },
    LoadAccessFault { addr: VirtAddr },
    StoreAddressMisaligned { addr: VirtAddr },
    StoreAccessFault { addr: VirtAddr },
    
    // System calls
    EnvironmentCallFromU,  // ecall from user mode
    EnvironmentCallFromS,  // ecall from supervisor mode
    
    // Page faults
    InstructionPageFault { addr: VirtAddr },
    LoadPageFault { addr: VirtAddr },
    StorePageFault { addr: VirtAddr },
    
    // Interrupts
    TimerInterrupt,
    ExternalInterrupt,
}

/// Trait that the kernel implements to handle traps
pub trait TrapHandler: Send {
    /// Handle a trap. Returns the address to resume execution.
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut Memory,
    ) -> Result<VirtAddr, TrapError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TrapError {
    #[error("unhandled trap: {0:?}")]
    Unhandled(TrapCause),
    
    #[error("trap handler panicked: {0}")]
    HandlerPanic(String),
}
```

#### Device Interface

```rust
// devices/mod.rs

pub trait Device: Send {
    /// Device name (for debugging)
    fn name(&self) -> &str;
    
    /// Read a 32-bit word from device register
    fn read(&mut self, offset: u32) -> Result<u32, DeviceError>;
    
    /// Write a 32-bit word to device register
    fn write(&mut self, offset: u32, value: u32) -> Result<(), DeviceError>;
    
    /// Called on each VM step (for timers, etc.)
    fn tick(&mut self) -> Result<Option<DeviceInterrupt>, DeviceError>;
}

pub struct DeviceInterrupt {
    pub device_name: String,
    pub irq_number: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("invalid register offset: {0:#x}")]
    InvalidOffset(u32),
    
    #[error("device not ready")]
    NotReady,
    
    #[error("I/O error: {0}")]
    Io(String),
}
```

#### Memory Management Unit

```rust
// mmu.rs

/// Sv32 page table entry
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    entry: u32,
}

impl PageTableEntry {
    pub fn new() -> Self;
    pub fn is_valid(&self) -> bool;
    pub fn is_readable(&self) -> bool;
    pub fn is_writable(&self) -> bool;
    pub fn is_executable(&self) -> bool;
    pub fn is_user_accessible(&self) -> bool;
    pub fn is_dirty(&self) -> bool;
    pub fn is_accessed(&self) -> bool;
    pub fn ppn(&self) -> PhysPageNum;
    
    pub fn set_valid(&mut self, valid: bool);
    pub fn set_readable(&mut self, readable: bool);
    pub fn set_writable(&mut self, writable: bool);
    pub fn set_executable(&mut self, executable: bool);
    pub fn set_user_accessible(&mut self, user: bool);
    pub fn set_ppn(&mut self, ppn: PhysPageNum);
}

pub struct Mmu {
    // MMU state
    satp: u32,  // Supervisor Address Translation and Protection
    enabled: bool,
}

impl Mmu {
    pub fn new() -> Self;
    
    /// Translate virtual address to physical address
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        access_type: AccessType,
        privilege: PrivilegeMode,
        memory: &Memory,
    ) -> Result<PhysAddr, PageFault>;
    
    pub fn enable(&mut self, page_table_root: PhysPageNum);
    pub fn disable(&mut self);
}

#[derive(Debug, Clone, Copy)]
pub enum AccessType {
    Instruction,
    Load,
    Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeMode {
    User,
    Supervisor,
}
```

#### Error Types

```rust
// error.rs

#[derive(Debug, thiserror::Error)]
pub enum VmError {
    #[error("memory error: {0}")]
    Memory(#[from] MemoryError),
    
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),
    
    #[error("trap error: {0}")]
    Trap(#[from] TrapError),
    
    #[error("device error: {0}")]
    Device(#[from] DeviceError),
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("out of bounds access: {0:#x}")]
    OutOfBounds(u32),
    
    #[error("misaligned access: address {addr:#x}, alignment {alignment}")]
    Misaligned { addr: u32, alignment: u32 },
    
    #[error("access violation: tried to {op} at {addr:#x}")]
    AccessViolation { op: &'static str, addr: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid opcode: {0:#x}")]
    InvalidOpcode(u32),
    
    #[error("invalid instruction encoding: {0:#x}")]
    InvalidEncoding(u32),
}
```

---

### Component 2: ferrous-kernel (Operating System Kernel)


#### Module Structure

```text
ferrous-kernel/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public kernel API
    ├── main.rs             # Kernel entry point
    ├── types.rs            # Common kernel types (handles, etc.)
    ├── syscall.rs          # System call dispatcher
    ├── allocator.rs        # Kernel heap allocator setup
    │
    ├── thread/
    │   ├── mod.rs
    │   ├── tcb.rs          # Thread Control Block
    │   ├── scheduler.rs    # Scheduling algorithms
    │   ├── context.rs      # Context switching
    │   └── tests.rs
    │
    ├── sync/
    │   ├── mod.rs
    │   ├── semaphore.rs
    │   ├── lock.rs
    │   ├── condvar.rs
    │   └── tests.rs
    │
    ├── vm/
    │   ├── mod.rs
    │   ├── address_space.rs  # Per-process address space
    │   ├── page_table.rs     # Page table management
    │   ├── frame_allocator.rs # Physical frame allocation
    │   └── tests.rs
    │
    ├── fs/
    │   ├── mod.rs
    │   ├── inode.rs
    │   ├── directory.rs
    │   ├── file.rs
    │   ├── buffer_cache.rs
    │   └── tests.rs
    │
    ├── net/
    │   ├── mod.rs
    │   ├── socket.rs
    │   ├── packet.rs
    │   ├── link_layer.rs
    │   ├── transport.rs
    │   └── tests.rs
    │
    └── error.rs
```

#### Kernel Initialization

```rust
// lib.rs

use ferrous_vm::{TrapHandler, TrapCause, Cpu, Memory, VirtAddr};

pub struct Kernel {
    thread_manager: ThreadManager,
    sync_manager: SyncManager,
    vm_manager: VirtualMemoryManager,
    fs_manager: FileSystemManager,
    net_manager: NetworkManager,
}

impl Kernel {
    pub fn new() -> Result<Self, KernelError>;
    pub fn boot(&mut self) -> Result<(), KernelError>;
}

impl TrapHandler for Kernel {
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut Memory,
    ) -> Result<VirtAddr, TrapError> {
        match cause {
            TrapCause::EnvironmentCallFromU => {
                self.handle_syscall(cpu, memory)
            }
            TrapCause::TimerInterrupt => {
                self.handle_timer_interrupt(cpu, memory)
            }
            TrapCause::LoadPageFault { addr } |
            TrapCause::StorePageFault { addr } |
            TrapCause::InstructionPageFault { addr } => {
                self.handle_page_fault(addr, cause, cpu, memory)
            }
            _ => Err(TrapError::Unhandled(cause)),
        }
    }
}
```

#### System Call Interface

```rust
// syscall.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Syscall {
    // Thread management
    ThreadCreate { entry_point: VirtAddr, arg: u32 },
    ThreadExit { exit_code: i32 },
    ThreadYield,
    ThreadSleep { duration_ms: u32 },
    ThreadJoin { thread: ThreadHandle },
    
    // Synchronization
    SemCreate { initial_value: u32 },
    SemWait { sem: SemaphoreHandle },
    SemSignal { sem: SemaphoreHandle },
    SemDestroy { sem: SemaphoreHandle },
    
    LockCreate,
    LockAcquire { lock: LockHandle },
    LockRelease { lock: LockHandle },
    LockDestroy { lock: LockHandle },
    
    CondVarCreate,
    CondVarWait { cv: CondVarHandle, lock: LockHandle },
    CondVarSignal { cv: CondVarHandle },
    CondVarBroadcast { cv: CondVarHandle },
    CondVarDestroy { cv: CondVarHandle },
    
    // Memory management
    Mmap { addr: Option<VirtAddr>, length: usize, prot: Protection },
    Munmap { addr: VirtAddr, length: usize },
    Brk { addr: VirtAddr },
    
    // File system
    Open { path_ptr: VirtAddr, path_len: usize, flags: OpenFlags },
    Close { fd: FileDescriptor },
    Read { fd: FileDescriptor, buf_ptr: VirtAddr, count: usize },
    Write { fd: FileDescriptor, buf_ptr: VirtAddr, count: usize },
    Seek { fd: FileDescriptor, offset: i64, whence: SeekWhence },
    Unlink { path_ptr: VirtAddr, path_len: usize },
    
    Mkdir { path_ptr: VirtAddr, path_len: usize },
    Rmdir { path_ptr: VirtAddr, path_len: usize },
    Readdir { fd: FileDescriptor, buf_ptr: VirtAddr, buf_len: usize },
    
    // Networking
    Socket { domain: SocketDomain, socket_type: SocketType },
    Bind { socket: SocketHandle, addr_ptr: VirtAddr },
    Connect { socket: SocketHandle, addr_ptr: VirtAddr },
    Listen { socket: SocketHandle, backlog: u32 },
    Accept { socket: SocketHandle },
    Send { socket: SocketHandle, buf_ptr: VirtAddr, len: usize, flags: u32 },
    Recv { socket: SocketHandle, buf_ptr: VirtAddr, len: usize, flags: u32 },
    
    // I/O
    ConsoleWrite { buf_ptr: VirtAddr, len: usize },
    ConsoleRead { buf_ptr: VirtAddr, len: usize },
}

impl Syscall {
    /// Decode syscall from CPU registers (a0-a7)
    pub fn from_registers(cpu: &Cpu) -> Result<Self, SyscallError>;
    
    /// Encode return value into CPU registers
    pub fn encode_result(result: SyscallResult, cpu: &mut Cpu);
}

pub type SyscallResult = Result<SyscallReturn, SyscallError>;

#[derive(Debug)]
pub enum SyscallReturn {
    Success,
    Handle(u32),  // Generic handle (thread, fd, socket, etc.)
    Value(i64),   // Numeric return value
    Pointer(VirtAddr),
}

#[derive(Debug, thiserror::Error)]
pub enum SyscallError {
    #[error("invalid syscall number: {0}")]
    InvalidSyscallNumber(u32),
    
    #[error("invalid argument")]
    InvalidArgument,
    
    #[error("permission denied")]
    PermissionDenied,
    
    #[error("resource not found")]
    NotFound,
    
    #[error("resource already exists")]
    AlreadyExists,
    
    #[error("out of memory")]
    OutOfMemory,
    
    #[error("operation would block")]
    WouldBlock,
}
```

#### Thread Management

```rust
// thread/tcb.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked { reason: BlockReason },
    Terminated { exit_code: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    WaitingOnSemaphore(SemaphoreHandle),
    WaitingOnLock(LockHandle),
    WaitingOnCondVar(CondVarHandle),
    Sleeping { wake_time: SimulatedInstant },
    WaitingOnThread(ThreadHandle),
}

pub struct ThreadControlBlock {
    handle: ThreadHandle,
    state: ThreadState,
    priority: Priority,
    
    // Saved context (registers)
    context: SavedContext,
    
    // Address space
    page_table_root: Option<PhysPageNum>,
    
    // Kernel stack
    kernel_stack: KernelStack,
    
    // Statistics
    creation_time: SimulatedInstant,
    total_cpu_time: Duration,
}

#[derive(Debug, Clone, Copy)]
pub struct SavedContext {
    pub pc: u32,
    pub regs: [u32; 32],
    // ... other saved state
}

pub struct Priority(u8);

impl Priority {
    pub const MIN: Priority = Priority(0);
    pub const DEFAULT: Priority = Priority(127);
    pub const MAX: Priority = Priority(255);
}
```

```rust
// thread/scheduler.rs

pub trait Scheduler {
    /// Select next thread to run
    fn schedule(&mut self) -> Option<ThreadHandle>;
    
    /// Add thread to ready queue
    fn enqueue(&mut self, thread: ThreadHandle);
    
    /// Remove thread from ready queue
    fn dequeue(&mut self, thread: ThreadHandle) -> bool;
    
    /// Called on timer tick
    fn tick(&mut self);
}

/// Round-robin scheduler (initial implementation)
pub struct RoundRobinScheduler {
    ready_queue: VecDeque<ThreadHandle>,
    time_quantum_ms: u64,
    current_quantum_remaining: u64,
}

/// Priority-based scheduler (assignment 3)
pub struct PriorityScheduler {
    queues: [VecDeque<ThreadHandle>; 256],  // One per priority level
}

/// Multi-Level Feedback Queue (assignment 3)
pub struct MlfqScheduler {
    queues: Vec<VecDeque<ThreadHandle>>,
    time_quanta: Vec<u64>,
    priorities: HashMap<ThreadHandle, usize>,
}
```

```rust
// thread/mod.rs

pub struct ThreadManager {
    threads: HashMap<ThreadHandle, ThreadControlBlock>,
    scheduler: Box<dyn Scheduler>,
    current_thread: Option<ThreadHandle>,
    next_handle: u32,
}

impl ThreadManager {
    pub fn new(scheduler: Box<dyn Scheduler>) -> Self;
    
    pub fn create_thread(
        &mut self,
        entry_point: VirtAddr,
        arg: u32,
    ) -> Result<ThreadHandle, ThreadError>;
    
    pub fn exit_current_thread(&mut self, exit_code: i32) -> Re

#### Virtual Memory Management

```rust
// vm/address_space.rs

pub struct AddressSpace {
    page_table_root: PhysPageNum,
    regions: Vec<MemoryRegion>,
}

pub struct MemoryRegion {
    start: VirtAddr,
    end: VirtAddr,
    permissions: Protection,
    region_type: RegionType,
}

#[derive(Debug, Clone, Copy)]
pub struct Protection {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RegionType {
    Code,
    Data,
    Heap,
    Stack,
    Mmap,
}

impl AddressSpace {
    pub fn new() -> Result<Self, VmError>;
    
    pub fn map_region(
        &mut self,
        region: MemoryRegion,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<(), VmError>;
    
    pub fn unmap_region(&mut self, start: VirtAddr, len: usize) -> Result<(), VmError>;
    
    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        access_type: AccessType,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<(), PageFaultError>;
}
```

```rust
// vm/frame_allocator.rs

pub struct FrameAllocator {
    free_frames: VecDeque<PhysPageNum>,
    total_frames: usize,
    allocated_frames: usize,
}

impl FrameAllocator {
    pub fn new(memory_size: usize) -> Self;
    pub fn allocate(&mut self) -> Result<PhysPageNum, OutOfMemory>;
    pub fn deallocate(&mut self, frame: PhysPageNum);
    pub fn available(&self) -> usize;
}
```

---

### Component 3: ferrous-user (User Program Library)

#### Module Structure

```text
ferrous-user/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── syscall.rs      # Raw syscall interface
    ├── thread.rs       # Thread API
    ├── sync.rs         # Synchronization API
    ├── io.rs           # I/O operations
    ├── fs.rs           # File system API
    └── net.rs          # Network API
```

#### Public API Design

```rust
// lib.rs
#![no_std]

pub mod thread;
pub mod sync;
pub mod io;
pub mod fs;
pub mod net;

// Re-exports for convenience
pub use thread::{spawn, yield_now, sleep, exit};
pub use sync::{Semaphore, Mutex, CondVar};
pub use io::{print, println, read_line};
```

```rust
// thread.rs

use crate::syscall;
use core::time::Duration;

pub struct ThreadHandle(u32);

pub fn spawn<F>(f: F) -> Result<ThreadHandle, Error>
where
    F: FnOnce() + Send + 'static,
{
    // Implementation uses syscall::thread_create
}

pub fn yield_now() {
    syscall::thread_yield();
}

pub fn sleep(duration: Duration) {
    let ms = duration.as_millis() as u32;
    syscall::thread_sleep(ms);
}

pub fn exit(code: i32) -> ! {
    syscall::thread_exit(code);
    unreachable!()
}
```

```rust
// sync.rs

pub struct Semaphore {
    handle: u32,
}

impl Semaphore {
    pub fn new(initial_value: u32) -> Result<Self, Error>;
    pub fn wait(&self) -> Result<(), Error>;
    pub fn signal(&self) -> Result<(), Error>;
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        // syscall::sem_destroy
    }
}

pub struct Mutex {
    handle: u32,
}

impl Mutex {
    pub fn new() -> Result<Self, Error>;
    pub fn lock(&self) -> Result<MutexGuard, Error>;
}

pub struct MutexGuard<'a> {
    mutex: &'a Mutex,
}

impl Drop for MutexGuard<'_> {
    fn drop(&mut self) {
        // syscall::lock_release
    }
}
```

```rust
// io.rs

pub fn print(s: &str) {
    syscall::console_write(s.as_ptr(), s.len());
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut buf = StringBuffer::new();
        write!(&mut buf, $($arg)*).unwrap();
        $crate::io::print(buf.as_str());
        $crate::io::print("\n");
    }};
}
```

---

### Component 4: ferrous-runtime (Runtime & Tools)

#### ELF Loader

```rust
// runtime/src/loader.rs

pub struct ProgramLoader {
    vm: VirtualMachine,
}

impl ProgramLoader {
    pub fn load_elf(&mut self, elf_bytes: &[u8]) -> Result<VirtAddr, LoaderError>;
}
```

#### Debugger

```rust
// runtime/src/debugger.rs

pub struct Debugger {
    vm: VirtualMachine,
    breakpoints: HashSet<VirtAddr>,
}

impl Debugger {
    pub fn add_breakpoint(&mut self, addr: VirtAddr);
    pub fn remove_breakpoint(&mut self, addr: VirtAddr);
    pub fn step(&mut self) -> Result<StepResult, VmError>;
    pub fn continue_execution(&mut self) -> Result<ExitReason, VmError>;
    pub fn inspect_register(&self, reg: Register) -> u32;
    pub fn inspect_memory(&self, addr: VirtAddr, len: usize) -> Result<Vec<u8>, MemoryError>;
}
```

---

## Implementation Roadmap

### Iteration 1: Hello World (Weeks 1-3)

**Goal**: Single-threaded program prints "Hello World"

**Components to Build**:

1. **Project Setup**
   - [x] Create Cargo workspace
   - [x] Set up directory structure
   - [x] Configure .gitignore
   - [x] Add dependencies (thiserror, log)

2. **ferrous-vm (Minimal)**
   - [ ] Basic CPU struct with registers
   - [ ] Instruction enum (subset: LUI, ADDI, AUIPC, JAL, JALR, ECALL)
   - [ ] Instruction decoder for minimal set
   - [ ] Execute function with pattern match
   - [ ] Flat memory (just Vec<u8>)
   - [ ] Console device (write-only)
   - [ ] Trap handling for ECALL

3. **ferrous-kernel (Minimal)**
   - [ ] Kernel struct implementing TrapHandler
   - [ ] Single syscall: ConsoleWrite
   - [ ] No threading yet - just execute program

4. **ferrous-runtime**
   - [ ] Simple ELF loader (load segments into memory)
   - [ ] Set PC to entry point
   - [ ] Run VM loop

5. **ferrous-user**
   - [ ] print() function
   - [ ] println!() macro

6. **Example Program**
   - [ ] hello-world example
   - [ ] Custom target spec for riscv32ima-unknown-none-elf
   - [ ] Linker script
   - [ ] Build script

**Success Criteria**:
- `cargo run --example hello-world` prints "Hello World"
- Clean compilation with no warnings
- Basic integration test passes

---

### Iteration 2: Threading Basics (Weeks 4-6)

**Goal**: Multiple threads with cooperative scheduling

**New Components**:

1. **ferrous-vm**
   - [ ] Complete RV32I instruction set
   - [ ] Add more load/store instructions
   - [ ] Add branch instructions

2. **ferrous-kernel**
   - [ ] ThreadControlBlock structure
   - [ ] Thread state enum
   - [ ] SavedContext for register state
   - [ ] ThreadManager with HashMap
   - [ ] Context switch implementation
   - [ ] RoundRobinScheduler
   - [ ] Syscalls: ThreadCreate, ThreadExit, ThreadYield

3. **ferrous-user**
   - [ ] spawn() function
   - [ ] yield\_now() function
   - [ ] exit() function

4. **Tests**
   - [ ] Multi-threaded example
   - [ ] Context switch test
   - [ ] Thread lifecycle test

**Success Criteria**:
- Program creates 3 threads
- Threads yield cooperatively
- All threads execute and print messages
- Threads terminate cleanly

---

### Iteration 3: Preemptive Scheduling (Weeks 7-8)

**Goal**: Timer interrupts enable preemption

**New Components**:

1. **ferrous-vm**
   - [ ] Timer device implementation
   - [ ] Interrupt handling (separate from exceptions)
   - [ ] Timer interrupt delivery

2. **ferrous-kernel**
   - [ ] Timer interrupt handler
   - [ ] Preemptive context switch
   - [ ] Time quantum tracking
   - [ ] Syscall: ThreadSleep
   - [ ] Sleeping threads management

3. **Tests**
   - [ ] Preemption test (CPU-bound threads)
   - [ ] Sleep test
   - [ ] Timer accuracy test

**Success Criteria**:
- Threads run without yielding
- Time-slicing works correctly
- Sleep with correct wake time

---

### Iteration 4: Synchronization (Weeks 9-11)

**Goal**: Locks, semaphores, condition variables

**New Components**:

1. **ferrous-kernel**
   - [ ] Semaphore implementation
   - [ ] Lock implementation  
   - [ ] CondVar implementation
   - [ ] Wait queue management
   - [ ] Block/unblock thread operations
   - [ ] All sync syscalls

2. **ferrous-user**
   - [ ] Semaphore API
   - [ ] Mutex API with RAII guards
   - [ ] CondVar API

3. **Tests & Examples**
   - [ ] Producer-consumer
   - [ ] Dining philosophers
   - [ ] Readers-writers
   - [ ] Bounded buffer

**Success Criteria**:
- All classic sync problems work
- No deadlocks
- No race conditions
- Clean shutdown

---

### Iteration 5: Virtual Memory (Weeks 12-15)

**Goal**: Page tables, demand paging, isolation

**New Components**:

1. **ferrous-vm**
   - [ ] Complete RV32M extension
   - [ ] MMU implementation (Sv32)
   - [ ] TLB simulation
   - [ ] Page fault exceptions

2. **ferrous-kernel**
   - [ ] AddressSpace structure
   - [ ] PageTable management
   - [ ] FrameAllocator
   - [ ] Page fault handler
   - [ ] Demand paging (lazy allocation)
   - [ ] Copy-on-write fork
   - [ ] Page replacement (FIFO, LRU, Clock)
   - [ ] Syscalls: mmap, munmap, brk

3. **Tests**
   - [ ] Page table walk test
   - [ ] Page fault test
   - [ ] COW test
   - [ ] Page replacement test
   - [ ] Memory isolation test

**Success Criteria**:
- Each process has isolated address space
- Demand paging works
- COW fork works
- Page replacement doesn't corrupt memory

---

### Iteration 6: File System (Weeks 16-19)

**Goal**: Persistent storage with files and directories

**New Components**:

1. **ferrous-vm**
   - [ ] Disk device (block device)
   - [ ] Block read/write operations

2. **ferrous-kernel**
   - [ ] Disk layout (superblock, inodes, data blocks)
   - [ ] Inode structure
   - [ ] Directory structure
   - [ ] File operations
   - [ ] Buffer cache
   - [ ] File descriptor table
   - [ ] All FS syscalls

3. **ferrous-user**
   - [ ] File API (open, read, write, close)
   - [ ] Directory API

4. **Tests & Examples**
   - [ ] File creation/deletion
   - [ ] Directory operations
   - [ ] Large file test
   - [ ] Stress test

**Success Criteria**:
- Create, write, read, delete files
- Directory hierarchy works
- Buffer cache improves performance

---

### Iteration 7: Networking (Weeks 20-23)

**Goal**: Communication between VMs

**New Components**:

1. **ferrous-vm**
   - [ ] Network device
   - [ ] Packet send/receive
   - [ ] Complete RV32A extension

2. **ferrous-kernel**
   - [ ] Packet structure
   - [ ] Link layer
   - [ ] Transport layer (reliable + unreliable)
   - [ ] Socket abstraction
   - [ ] All network syscalls

3. **ferrous-user**
   - [ ] Socket API

4. **Tests & Examples**
   - [ ] Echo server/client
   - [ ] Chat application
   - [ ] File transfer

**Success Criteria**:
- Two VMs communicate
- Reliable delivery works
- Socket API is usable

---

### Iterations 8-11: Polish, Testing, Assignments, Documentation (Weeks 24-34)

**Remaining Work**:

1. **Advanced Scheduling (Weeks 24-25)**
   - Priority scheduler
   - MLFQ scheduler
   - Performance metrics

2. **Polish & Tooling (Weeks 26-28)**
   - Interactive debugger
   - Execution tracer
   - Performance profiler
   - Code cleanup and refactoring

3. **Testing Framework (Weeks 29-30)**
   - Comprehensive test suite
   - Automated grading infrastructure
   - Performance benchmarks

4. **Assignments & Documentation (Weeks 31-34)**
   - Create assignment skeletons
   - Reference solutions
   - Complete documentation (mdBook)
   - Getting started guides
   - API documentation

---

## Testing Strategy

### Unit Tests

**Location**: `#[cfg(test)] mod tests` within each module

**Coverage**:
- VM instruction execution
- Page table operations
- Scheduler algorithms
- Synchronization primitives

**Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_instruction_decode_add() {
        let word = 0x003100b3; // add x1, x2, x3
        let inst = Instruction::decode(word).unwrap();
        assert_eq!(inst, Instruction::Add {
            rd: Register::new(1).unwrap(),
            rs1: Register::new(2).unwrap(),
            rs2: Register::new(3).unwrap(),
        });
    }
}
```

### Integration Tests

**Location**: `tests/` directory

**Coverage**:
- VM + Kernel integration
- Full syscall paths
- Multi-component interactions

**Example**:
```rust
// tests/thread_tests.rs
#[test]
fn test_thread_creation_and_execution() {
    let kernel = Kernel::new().unwrap();
    let vm = VirtualMachine::new(config, Box::new(kernel)).unwrap();
    let program = load_test_program("thread_test.elf");
    vm.load_program(&program).unwrap();
    let result = vm.run().unwrap();
    assert_eq!(result, ExitReason::Halt);
}
```

### Property-Based Testing

Use `proptest` for critical algorithms:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_page_table_walk_inverse(vpn in 0u32..0x100000) {
        let mut pt = PageTable::new();
        let ppn = allocate_frame();
        pt.map(VirtPageNum(vpn), ppn).unwrap();
        assert_eq!(pt.translate(VirtPageNum(vpn)).unwrap(), ppn);
    }
}
```

### End-to-End Tests

Run complete user programs:

```rust
#[test]
fn test_producer_consumer() {
    let output = run_user_program("producer_consumer.elf");
    assert!(output.contains("Producer: produced 100 items"));
    assert!(output.contains("Consumer: consumed 100 items"));
}
```

---

## Assignment Structure

### Assignment Integration Pattern

Students implement traits in separate crates:

```
assignments/
├── assignment-1-threads/
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs       # Student implements Scheduler trait
│   └── tests/
│       ├── public.rs    # Visible to students
│       └── grading.rs   # Instructor-only (feature-gated)
```

```rust
// assignment-1-threads/src/lib.rs

use ferrous_kernel::thread::{Scheduler, ThreadHandle};
use std::collections::VecDeque;

pub struct StudentScheduler {
    // TODO: Add your fields here
}

impl Scheduler for StudentScheduler {
    fn schedule(&mut self) -> Option<ThreadHandle> {
        todo!("Assignment 1: Implement round-robin scheduling")
    }
    
    fn enqueue(&mut self, thread: ThreadHandle) {
        todo!("Assignment 1: Add thread to ready queue")
    }
    
    fn dequeue(&mut self, thread: ThreadHandle) -> bool {
        todo!("Assignment 1: Remove thread from ready queue")
    }
    
    fn tick(&mut self) {
        todo!("Assignment 1: Handle timer tick")
    }
}
```

**Student workflow**:
```console
cd assignments/assignment-1-threads
cargo test                # Run public tests
cargo test --features grading  # (won't work - feature disabled for students)
```

**Instructor workflow**:
```console
cd assignments/assignment-1-threads
cargo test --features grading  # Run all tests including hidden ones
```

---

## Conclusion

This architecture specification provides a complete blueprint for building **Ferrous**, a professional-grade educational operating system in Rust.

### Key Design Decisions Summary

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Error Handling** | `Result<T, E>` everywhere | Type-safe, idiomatic, no panics |
| **Type Safety** | Newtypes for all domain concepts | Compiler-enforced correctness |
| **Architecture** | Trait-based layers | Testable, modular, extensible |
| **Memory** | Typed address wrappers | Cannot mix phys/virt addresses |
| **Concurrency** | Single-threaded simulation | Deterministic, easier to debug |
| **Testing** | Unit + integration + property | Comprehensive quality assurance |
| **ISA** | RISC-V RV32IMA | Modern, educational, realistic |
| **Execution** | Interpreted | Simple, debuggable, sufficient |
| **VM/Kernel** | Trait-based TrapHandler | Clean separation of concerns |
| **Devices** | Trait-based Device interface | Polymorphic, extensible |
| **Syscalls** | Enum with typed variants | Type-safe, exhaustive matching |
| **Collections** | Safe types from alloc | Idiomatic, maintainable |
| **IDs** | NonZeroU32 newtypes | Niche optimization, type safety |
| **Modules** | Short names (vm, fs, net) | Concise, Rust conventions |
| **Errors** | Per-module error types | Clear ownership, composable |
| **Visibility** | Minimal pub, explicit pub(crate) | Conservative API surface |
| **Documentation** | rustdoc + mdBook | Standard Rust workflow |
| **Assignments** | Trait implementation pattern | Clean separation, cannot break framework |
| **Solutions** | In-repo separate directory | Easy to maintain and sync |

### Architecture Strengths

1. **Maintainable**: Idiomatic Rust, clear patterns, no clever code
2. **Type-Safe**: Compiler prevents entire classes of bugs
3. **Testable**: Every component independently testable
4. **Educational**: Clear, well-documented, instructive code
5. **Extensible**: Trait-based design allows future addi
