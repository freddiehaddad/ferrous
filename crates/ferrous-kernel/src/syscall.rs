use crate::error::SyscallError;
use crate::types::ThreadHandle;
use ferrous_vm::{Cpu, Register, VirtAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Syscall {
    // I/O
    ConsoleWrite {
        fd: u32,
        buf_ptr: VirtAddr,
        len: usize,
    },
    Exit {
        code: i32,
    },

    // Threading
    ThreadCreate {
        entry_point: VirtAddr,
        stack_top: u32,
    },
    ThreadYield,

    // Synchronization
    MutexCreate,
    MutexAcquire {
        id: u32,
    },
    MutexRelease {
        id: u32,
    },

    // Memory
    Sbrk {
        increment: i32,
    },

    // Block Device (Temporary Debug)
    BlockRead {
        sector: u32,
        buf_ptr: VirtAddr,
    },
}

#[derive(Debug)]
pub enum SyscallReturn {
    Success,
    Handle(u32),
    Value(i64),
    Pointer(VirtAddr),
}

impl Syscall {
    pub fn from_registers(cpu: &Cpu) -> Result<Self, SyscallError> {
        let a0 = cpu.read_reg(Register::new(10).unwrap());
        let a1 = cpu.read_reg(Register::new(11).unwrap());
        let a2 = cpu.read_reg(Register::new(12).unwrap());
        let a7 = cpu.read_reg(Register::new(17).unwrap()); // syscall number

        match a7 {
            64 => Ok(Syscall::ConsoleWrite {
                fd: a0,
                buf_ptr: VirtAddr::new(a1),
                len: a2 as usize,
            }),
            93 => Ok(Syscall::Exit { code: a0 as i32 }),
            101 => Ok(Syscall::ThreadYield),
            102 => Ok(Syscall::ThreadCreate {
                entry_point: VirtAddr::new(a0),
                stack_top: a1,
            }),
            110 => Ok(Syscall::MutexCreate),
            111 => Ok(Syscall::MutexAcquire { id: a0 }),
            112 => Ok(Syscall::MutexRelease { id: a0 }),
            200 => Ok(Syscall::BlockRead {
                sector: a0,
                buf_ptr: VirtAddr::new(a1),
            }),
            214 => Ok(Syscall::Sbrk {
                increment: a0 as i32,
            }),
            _ => Err(SyscallError::InvalidSyscallNumber(a7)),
        }
    }

    pub fn encode_result(result: Result<SyscallReturn, SyscallError>, cpu: &mut Cpu) {
        let a0 = Register::new(10).unwrap();
        match result {
            Ok(SyscallReturn::Success) => cpu.write_reg(a0, 0),
            Ok(SyscallReturn::Value(v)) => cpu.write_reg(a0, v as u32),
            Ok(SyscallReturn::Handle(h)) => cpu.write_reg(a0, h),
            Ok(SyscallReturn::Pointer(p)) => cpu.write_reg(a0, p.val()),
            Err(_) => {
                cpu.write_reg(a0, u32::MAX);
            }
        }
    }
}
