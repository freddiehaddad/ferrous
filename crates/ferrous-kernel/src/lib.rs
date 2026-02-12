pub mod error;
pub mod syscall;

use crate::error::KernelError;
use ferrous_vm::{Cpu, Memory, TrapCause, TrapError, TrapHandler, VirtAddr};
use log::{debug, info};

pub struct Kernel {
    // For now, minimal state
}

impl Kernel {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {})
    }

    pub fn handle_syscall(
        &mut self,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError> {
        // Decode syscall
        let syscall = syscall::Syscall::from_registers(cpu)
            .map_err(|e| TrapError::HandlerPanic(format!("Syscall decode error: {:?}", e)))?;

        debug!("Syscall: {:?}", syscall);

        let result = match syscall {
            syscall::Syscall::ConsoleWrite {
                fd: _,
                buf_ptr,
                len,
            } => {
                // Read string from memory
                let mut buf = Vec::with_capacity(len);
                for i in 0..len {
                    // This is a slow way to read, but safe for now
                    let byte = memory
                        .read_byte(ferrous_vm::PhysAddr::new(buf_ptr.val() + i as u32))
                        .map_err(|e| TrapError::HandlerPanic(format!("Memory error: {:?}", e)))?;
                    buf.push(byte);
                }

                let s = String::from_utf8_lossy(&buf);
                print!("{}", s); // Print to host console

                Ok(syscall::SyscallReturn::Success)
            }
            syscall::Syscall::Exit { code } => {
                info!("Thread/Process Exit: {}", code);
                // We don't have threads yet, so just halt VM?
                // But wait, Kernel shouldn't halt VM directly.
                // It just returns to VM.
                // But if we return, VM continues.
                // We need to signal VM to stop.
                // The VM checks StepResult.
                // But here we are in handle_trap, returning VirtAddr.

                // If the syscall is Exit, we probably want to create a Trap that VM understands as Exit.
                // Or we loop forever in Kernel? No.

                // The VM loop:
                // match self.step() { Ok(Trap) => handle_trap() ... }
                // If handle_trap returns, it updates PC and continues.

                // We need a mechanism to tell VM to stop.
                // Currently `handle_trap` returns `Result<VirtAddr, TrapError>`.
                // Maybe we can return a special address or error?
                // Or we change `TrapHandler` to return `TrapResult` enum.

                // ARCHITECTURE.md says `handle_trap` returns `Result<VirtAddr, TrapError>`.
                // So we can't signal exit easily.

                // Hack: loop forever at current PC?
                // Better: Add `Ebreak` or similar that VM handles as Breakpoint/Exit.
                // But `handle_syscall` is called *because* of Ecall.

                // If we want to exit, we can't just return.
                // The VM `run()` loop runs until `StepResult::Exit`.
                // `StepResult::Exit` comes from `step()` logic.
                // `step()` executes `Ecall` -> returns `StepResult::Trap`.
                // Then `run()` calls `handle_trap`.

                // If `handle_trap` returns OK, execution continues.

                // We need to modify `TrapHandler` or `run()` loop to support exit from trap.
                // But `ARCHITECTURE.md` spec is fixed for now? I can modify it if I want as I'm the architect.

                // Let's modify `TrapHandler` trait in `ferrous-vm/src/trap.rs` to allow signaling exit.
                // But that requires changing `ferrous-vm`.
                // Or we can return a special Error that is not really an error but an Exit signal.
                // But `TrapError` variants are `Unhandled` or `HandlerPanic`.

                // Let's add `TrapHandled::Exit` variant if I change the return type.

                // For now, let's just loop locally in `handle_syscall`? No, that blocks the thread.

                // I'll stick to printing "Exit" and returning to next instruction, effectively ignoring exit for now,
                // or just panic to stop.
                // Since this is Iteration 1, panic is acceptable to stop execution if "Exit" syscall is called.
                // Or better: `Err(TrapError::HandlerPanic("Program Exited".into()))`
                Err(TrapError::HandlerPanic(format!(
                    "Program Exited with code {}",
                    code
                )))
            }
        };

        // Encode result
        match result {
            Ok(val) => syscall::Syscall::encode_result(Ok(val), cpu),
            Err(e) => {
                // If it's a panic/fatal error, we should probably return it as a TrapError to stop the VM
                // But encode_result expects SyscallError.
                // Let's just log and return the error to stop VM.
                return Err(e);
            }
        }

        // Resume at next instruction (epc + 4)
        Ok(VirtAddr::new(cpu.pc + 4))
    }
}

impl TrapHandler for Kernel {
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError> {
        match cause {
            TrapCause::EnvironmentCallFromU | TrapCause::EnvironmentCallFromS => {
                self.handle_syscall(cpu, memory)
            }
            _ => Err(TrapError::Unhandled(cause)),
        }
    }
}
