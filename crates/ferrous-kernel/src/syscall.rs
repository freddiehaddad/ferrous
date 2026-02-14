use crate::error::SyscallError;
use ferrous_vm::{Cpu, Register, VirtAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Syscall {
    // I/O
    FileWrite {
        fd: u32,
        buf_ptr: VirtAddr,
        len: usize,
    },
    ConsoleRead {
        fd: u32,
        buf_ptr: VirtAddr,
        len: usize,
    },
    Pipe {
        pipe_array_ptr: VirtAddr, // Pointer to array of 2 u32s (read_fd, write_fd)
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

    // Network
    Socket,
    Bind {
        fd: usize,
        ptr: VirtAddr,
        len: usize,
    },
    SendTo {
        fd: usize,
        buf_ptr: VirtAddr,
        len: usize,
        dest_ptr: VirtAddr,
        dest_len: usize,
    },
    RecvFrom {
        fd: usize,
        buf_ptr: VirtAddr,
        len: usize,
        src_ptr: VirtAddr,
        src_len_ptr: VirtAddr,
    },

    // Block Device (Temporary Debug)
    BlockRead {
        sector: u32,
        buf_ptr: VirtAddr,
    },

    // File System
    FileOpen {
        path_ptr: VirtAddr,
        path_len: usize,
    },
    FileRead {
        fd: u32,
        buf_ptr: VirtAddr,
        len: usize,
    },
    FileClose {
        fd: u32,
    },
    Exec {
        path_ptr: VirtAddr,
        path_len: usize,
        args_ptr: VirtAddr,
        args_len: usize,
    },
    WaitPid {
        pid: u32,
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
        let a3 = cpu.read_reg(Register::new(13).unwrap());
        let a4 = cpu.read_reg(Register::new(14).unwrap());
        let a7 = cpu.read_reg(Register::new(17).unwrap()); // syscall number

        match a7 {
            64 => Ok(Syscall::FileWrite {
                fd: a0,
                buf_ptr: VirtAddr::new(a1),
                len: a2 as usize,
            }),
            65 => Ok(Syscall::ConsoleRead {
                fd: a0,
                buf_ptr: VirtAddr::new(a1),
                len: a2 as usize,
            }),
            22 => Ok(Syscall::Pipe {
                pipe_array_ptr: VirtAddr::new(a0),
            }),
            56 => Ok(Syscall::FileOpen {
                path_ptr: VirtAddr::new(a0),
                path_len: a1 as usize,
            }),
            57 => Ok(Syscall::FileClose { fd: a0 }),
            59 => Ok(Syscall::Exec {
                path_ptr: VirtAddr::new(a0),
                path_len: a1 as usize,
                args_ptr: VirtAddr::new(a2),
                args_len: a3 as usize,
            }),
            260 => Ok(Syscall::WaitPid { pid: a0 }),
            63 => Ok(Syscall::FileRead {
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
            300 => Ok(Syscall::Socket),
            301 => Ok(Syscall::Bind {
                fd: a0 as usize,
                ptr: VirtAddr::new(a1),
                len: a2 as usize,
            }),
            302 => Ok(Syscall::SendTo {
                fd: a0 as usize,
                buf_ptr: VirtAddr::new(a1),
                len: a2 as usize,
                dest_ptr: VirtAddr::new(a3),
                dest_len: a4 as usize,
            }),
            303 => Ok(Syscall::RecvFrom {
                fd: a0 as usize,
                buf_ptr: VirtAddr::new(a1),
                len: a2 as usize,
                src_ptr: VirtAddr::new(a3),
                src_len_ptr: VirtAddr::new(a4),
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
