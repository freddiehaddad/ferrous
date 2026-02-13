# Ferrous

> A modern educational operating system framework in Rust

**Ferrous** is a pedagogical operating system designed to teach core OS concepts through hands-on implementation. Built entirely in Rust, it provides a RISC-V virtual machine and a partially-implemented kernel that students complete through structured assignments.

> **NOTE**: This project is currently in development!

## 🎯 Project Goals

- **Educational Excellence**: Teach threading, scheduling, synchronization, virtual memory, file systems, and networking
- **Modern Language**: Leverage Rust's type system to prevent entire classes of bugs
- **Professional Quality**: Idiomatic, maintainable code that serves as a reference implementation
- **Realistic Architecture**: RISC-V ISA with proper privilege levels and MMU simulation

## 🏗️ Architecture

```
┌──────────────────────────────────┐
│     User Programs (Guest)        │
│  Compiled to RISC-V ELF binaries │
│  (e.g., examples/hello-world)    │
└──────────────────────────────────┘
              ↓ syscalls (ecall)
┌──────────────────────────────────┐
│  RISC-V Simulator (ferrous-vm)   │
│  • RV32IMA instruction execution │
│  • Virtual memory (Sv32)         │
│  • Traps to Host Kernel          │
└──────────────────────────────────┘
              ↓ traps (Host Calls)
┌──────────────────────────────────┐
│      Ferrous Kernel (Host)       │
│  • Written in Rust (no_std)      │
│  • Runs natively on Host CPU     │
│  • Manages Guest VM State        │
│  • Implements Syscalls/Traps     │
└──────────────────────────────────┘
```

## ✨ Features

- **RISC-V RV32IMA Simulator**: Complete interpreter with M (multiply) and A (atomic) extensions
- **Host-Based Kernel**: Kernel runs as native code for easier debugging and iteration
- **Threading**: Cooperative and preemptive multithreading
- **Synchronization**: Semaphores, mutexes, condition variables
- **Virtual Memory**: Sv32 paging with demand paging and copy-on-write
- **File System**: Unix-like inode-based file system
- **Networking**: Simplified layered network stack with sockets
- **Type Safety**: Extensive use of newtypes to prevent programming errors

## 📚 Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Complete technical specification
- **API Documentation** - Generate with `cargo doc --open`

## 🚀 Quick Start

### Prerequisites

- Rust 1.80.0 or later
- Cargo
- RISC-V Target: `rustup target add riscv32i-unknown-none-elf`

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ferrous.git
cd ferrous

# Build the host tools (VM, CLI, Kernel)
cargo build -p ferrous-cli

# Build a user program (Target: riscv32i-unknown-none-elf)
cd examples/hello-world
cargo build
cd ../..
```

### Running a Program

Use the CLI to run the compiled user program:

```bash
# Run the hello-world example
cargo run -p ferrous-cli -- run target/riscv32i-unknown-none-elf/debug/hello-world
```

### Your First Program

### Your First Program

```rust
// examples/hello-world/src/main.rs
#![no_std]
#![no_main]

use ferrous_user::{print, println, exit};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello from Ferrous!");
    exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

## 📖 Assignments

Ferrous includes 6 comprehensive assignments covering core OS concepts:

1. **Threads & Scheduling** - Implement thread creation and round-robin scheduling
2. **Synchronization** - Build semaphores, locks, and condition variables
3. **Advanced Scheduling** - Priority scheduling and MLFQ
4. **Virtual Memory** - Page tables, demand paging, and page replacement
5. **File System** - Inodes, directories, and buffer cache
6. **Networking** - Protocol layers and socket API

Each assignment includes:
- Clear specification and learning objectives
- Starter code with trait definitions
- Public tests for immediate feedback
- Hidden grading tests (instructors only)

## 🛠️ Development Status

**Current Status**: Reference Kernel Implementation (Polishing Phase)

### Implementation Roadmap

- [x] **Iteration 1**: Hello World (Completed)
- [x] **Iteration 2**: Threading Basics (Completed)
- [x] **Iteration 3**: Preemptive Scheduling (Completed)
- [x] **Iteration 4**: Synchronization & Drivers (Completed)
- [x] **Iteration 5**: Virtual Memory (Completed)
- [x] **Iteration 6**: File System & Pipes (Reference Implemented)
- [ ] **Iteration 7**: Networking (Planned)
- [ ] **Iteration 8-11**: Polish, Testing, Documentation (In Progress)

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed iteration plans.

## 🏛️ Design Principles

### Type Safety First
Every domain concept has its own type:
```rust
pub struct ThreadHandle(NonZeroU32);  // Cannot confuse with other IDs
pub struct VirtAddr(u32);              // Cannot mix with PhysAddr
pub struct PhysAddr(u32);              // Compiler enforces correctness
```

### Error Handling
No panics in production code - all failures are explicit:
```rust
pub fn create_thread(&mut self, entry: VirtAddr) -> Result<ThreadHandle, ThreadError>;
```

### Clear Boundaries
Trait-based abstractions between components:
```rust
pub trait TrapHandler {
    fn handle_trap(&mut self, cause: TrapCause, cpu: &mut Cpu) -> Result<VirtAddr, TrapError>;
}
```

## 🧪 Testing

Ferrous uses multiple testing strategies:

```bash
# Unit tests (per module)
cargo test --lib

# Integration tests (cross-component)
cargo test --test '*'

# Specific assignment tests
cargo test --package assignment-1-threads

# Run with logging
RUST_LOG=debug cargo test
```

## 🤝 Contributing

This project is currently in the design and initial implementation phase. Contributions will be welcome once the core architecture is established.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the original **Nachos** educational OS (UC Berkeley)
- RISC-V architecture chosen for its simplicity and modern design
- Rust community for excellent OS development resources

## 📧 Contact

For questions or feedback, please open an issue on GitHub.

---

**Note**: This project is under active development. The architecture specification is complete and implementation is beginning. Check back for updates or star/watch the repository to follow progress.

## 🔗 Resources

- [RISC-V Specification](https://riscv.org/technical/specifications/)
- [Rust OS Development](https://os.phil-opp.com/)
- [Original Nachos](https://homes.cs.washington.edu/~tom/nachos/)
- [Writing an OS in Rust](https://os.phil-opp.com/)

---

**Built with ❤️ and Rust**
