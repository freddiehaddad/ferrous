use crate::syscall::{Syscall, SyscallReturn};
use crate::thread::ThreadManager;
use ferrous_vm::{Cpu, TrapError, VirtAddr};
use log::{debug, info};

pub fn handle_syscall(
    syscall: Syscall,
    thread_manager: &mut ThreadManager,
    cpu: &mut Cpu,
) -> Result<VirtAddr, TrapError> {
    match syscall {
        Syscall::ThreadCreate {
            entry_point,
            stack_top,
        } => {
            debug!(
                "ThreadCreate: entry={:?}, stack={:#x}",
                entry_point, stack_top
            );
            let result = thread_manager
                .create_thread(entry_point, stack_top)
                .map(|h| SyscallReturn::Handle(h.val()))
                .map_err(TrapError::HandlerPanic); // Should be SyscallError

            match result {
                Ok(val) => Syscall::encode_result(Ok(val), cpu),
                Err(e) => return Err(e),
            }
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::ThreadYield => {
            Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
            cpu.pc += 4;
            thread_manager.yield_thread(cpu);
            Ok(VirtAddr::new(cpu.pc))
        }
        Syscall::Exit { code } => {
            info!("Thread/Process Exit: {}", code);
            thread_manager.exit_current_thread(code);
            let scheduled = thread_manager.yield_thread(cpu);

            if !scheduled {
                return Err(TrapError::Halt);
            }
            Ok(VirtAddr::new(cpu.pc))
        }
        _ => Err(TrapError::HandlerPanic(
            "Invalid syscall for thread module".into(),
        )),
    }
}
