pub mod error;
pub mod syscall;
pub mod thread;
pub mod types;

use crate::error::KernelError;
use ferrous_vm::{Cpu, Memory, TrapCause, TrapError, TrapHandler, VirtAddr};
use log::{debug, info};
use thread::ThreadManager;

pub struct Kernel {
    thread_manager: ThreadManager,
}

impl Kernel {
    pub fn new() -> Result<Self, KernelError> {
        Ok(Self {
            thread_manager: ThreadManager::new(),
        })
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
                self.thread_manager.exit_current_thread(code);
                self.thread_manager.yield_thread(cpu);

                // If yield_thread returns, it means we switched to another thread or back to this one (unlikely if exited)
                // Actually exit_current_thread sets state to Terminated.
                // yield_thread picks next Ready thread.
                // If no threads ready, we might be stuck.
                // But let's assume there is at least one.

                // If no threads left, yield_thread might not update cpu?
                // yield_thread impl:
                // if let Some(next) = scheduler.schedule() ...
                // else ...

                // If we exit and no threads, we should probably stop VM.
                if self.thread_manager.current_thread.is_none() {
                    return Err(TrapError::Halt);
                }

                // If we switched context, cpu regs are updated to new thread.
                // We return Ok(pc) where pc is new thread's PC.
                // Wait, handle_trap returns VirtAddr.
                // Does it overwrite cpu.pc?
                // lib.rs in ferrous-vm: `self.cpu.pc = resume_addr.val();`
                // So we should return the new PC from cpu.pc?
                // Or just return `cpu.pc`?
                // Yes, yield_thread updates cpu.pc.
                // So we return `VirtAddr::new(cpu.pc)`.
                Ok(syscall::SyscallReturn::Success) // Value doesn't matter as we don't return to this thread
            }
            syscall::Syscall::ThreadCreate {
                entry_point,
                stack_top,
            } => {
                match self.thread_manager.create_thread(entry_point, stack_top) {
                    Ok(handle) => Ok(syscall::SyscallReturn::Handle(handle.val())),
                    Err(e) => Err(TrapError::HandlerPanic(e)), // Should be SyscallError
                }
            }
            syscall::Syscall::ThreadYield => {
                // We need to return Success to the YIELDING thread (current).
                // But we are switching away.
                // So we need to save the "return value" (0) into the saved context of the yielding thread.
                // encode_result writes to cpu regs.

                // Strategy:
                // 1. Write return value to current cpu regs (a0 = 0).
                // 2. Advance PC for current thread (so when it resumes, it's past ecall).
                // 3. Call yield_thread (saves current cpu including a0 and new PC, restores next).

                syscall::Syscall::encode_result(Ok(syscall::SyscallReturn::Success), cpu);
                cpu.pc += 4; // Advance PC before saving context

                self.thread_manager.yield_thread(cpu);

                // Now cpu has new thread's context.
                // We return new PC.
                // Note: If we switch back to this thread later, `cpu.pc` will be restored to `old_pc + 4`.
                // And `a0` will be 0.

                // Wait, if we return `Ok(VirtAddr::new(cpu.pc))`, the VM loop sets `cpu.pc = result`.
                // So we don't need to return `cpu.pc + 4` here if we already updated it in `yield_thread` (via restore).
                // Yes.

                // But wait, `handle_syscall` end returns `Ok(VirtAddr::new(cpu.pc + 4))` normally.
                // We must bypass that for Yield and Exit.

                // So I should refactor `handle_syscall` return logic.

                // Special handling for Yield/Exit: they modify control flow.
                Ok(syscall::SyscallReturn::Success)
            }
        };

        // If not Yield/Exit, we encode result and advance PC.
        // If Yield, we already encoded result and advanced PC (inside the match arm logic I wrote above).
        // If Exit, we don't return.

        // Let's refine the match to handle control flow.

        match syscall {
            syscall::Syscall::ThreadYield => {
                // Already handled in match arm:
                // 1. encode_result(Success)
                // 2. cpu.pc += 4
                // 3. yield_thread(cpu)
                // Return new PC
                Ok(VirtAddr::new(cpu.pc))
            }
            syscall::Syscall::Exit { .. } => {
                // yield_thread called. cpu updated.
                Ok(VirtAddr::new(cpu.pc))
            }
            _ => {
                // Normal syscall
                match result {
                    Ok(val) => syscall::Syscall::encode_result(Ok(val), cpu),
                    Err(e) => return Err(e),
                }
                Ok(VirtAddr::new(cpu.pc + 4))
            }
        }
    }
}

impl TrapHandler for Kernel {
    fn handle_trap(
        &mut self,
        cause: TrapCause,
        cpu: &mut Cpu,
        memory: &mut dyn Memory,
    ) -> Result<VirtAddr, TrapError> {
        // Ensure current thread is tracked (lazy init of main thread)
        self.thread_manager.ensure_current_thread(cpu);

        match cause {
            TrapCause::EnvironmentCallFromU | TrapCause::EnvironmentCallFromS => {
                self.handle_syscall(cpu, memory)
            }
            TrapCause::TimerInterrupt => {
                // Preemption: Yield current thread
                self.thread_manager.yield_thread(cpu);
                Ok(VirtAddr::new(cpu.pc))
            }
            _ => Err(TrapError::Unhandled(cause)),
        }
    }
}
