use alloc::collections::BTreeMap;
use ferrous_vm::{Cpu, TrapError, VirtAddr};

use crate::error::SyscallError;
use crate::sync::Mutex;
use crate::syscall::{Syscall, SyscallReturn};
use crate::thread::ThreadManager;

pub fn handle_syscall(
    syscall: Syscall,
    thread_manager: &mut ThreadManager,
    mutexes: &mut BTreeMap<u32, Mutex>,
    next_mutex_id: &mut u32,
    cpu: &mut Cpu,
) -> Result<VirtAddr, TrapError> {
    match syscall {
        Syscall::MutexCreate => {
            let id = *next_mutex_id;
            *next_mutex_id += 1;
            let mutex = Mutex::new(id);
            mutexes.insert(id, mutex);
            Syscall::encode_result(Ok(SyscallReturn::Handle(id)), cpu);
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        Syscall::MutexAcquire { id } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "MutexAcquire called without current thread".into(),
                ))?;

            if let Some(mutex) = mutexes.get_mut(&id) {
                if mutex.owner.is_none() {
                    mutex.owner = Some(current_handle);
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                    Ok(VirtAddr::new(cpu.pc + 4))
                } else {
                    mutex.wait_queue.push_back(current_handle);
                    thread_manager.block_current_thread();
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                    cpu.pc += 4;
                    thread_manager.yield_thread(cpu);
                    Ok(VirtAddr::new(cpu.pc))
                }
            } else {
                Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                Ok(VirtAddr::new(cpu.pc + 4))
            }
        }
        Syscall::MutexRelease { id } => {
            let current_handle = thread_manager
                .current_thread
                .ok_or(TrapError::HandlerPanic(
                    "MutexRelease called without current thread".into(),
                ))?;

            if let Some(mutex) = mutexes.get_mut(&id) {
                if mutex.owner == Some(current_handle) {
                    mutex.owner = None;
                    if let Some(next_owner) = mutex.wait_queue.pop_front() {
                        mutex.owner = Some(next_owner);
                        thread_manager.wake_thread(next_owner);
                    }
                    Syscall::encode_result(Ok(SyscallReturn::Success), cpu);
                } else {
                    Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
                }
            } else {
                Syscall::encode_result(Err(SyscallError::InvalidSyscallNumber(0)), cpu);
            }
            Ok(VirtAddr::new(cpu.pc + 4))
        }
        _ => Err(TrapError::HandlerPanic("Sync: Unhandled syscall".into())),
    }
}
