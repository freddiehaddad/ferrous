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
            // TODO: Assignment 2 - Implement Mutex acquisition
            // 1. Get current thread.
            // 2. Check if mutex exists.
            // 3. If mutex is free, take it.
            // 4. If mutex is held, add current thread to wait queue and block.
            todo!("Assignment 2: MutexAcquire");
        }
        Syscall::MutexRelease { id } => {
            // TODO: Assignment 2 - Implement Mutex release
            // 1. Get current thread.
            // 2. Check if mutex exists and is held by current thread.
            // 3. Release mutex (owner = None).
            // 4. If wait queue not empty, pop next thread and wake it.
            todo!("Assignment 2: MutexRelease");
        }
        Syscall::SemaphoreCreate { initial_count } => {
            // TODO: Assignment 2 - Implement Semaphore creation
            // 1. Create new Semaphore with initial_count.
            // 2. Store in semaphore map.
            // 3. Return handle.
            todo!("Assignment 2: SemaphoreCreate");
        }
        Syscall::SemaphoreDown { id } => {
            // TODO: Assignment 2 - Implement Semaphore Down (P)
            // 1. Check if semaphore exists.
            // 2. Decrement count if > 0.
            // 3. If count == 0, block current thread and add to wait queue.
            todo!("Assignment 2: SemaphoreDown");
        }
        Syscall::SemaphoreUp { id } => {
            // TODO: Assignment 2 - Implement Semaphore Up (V)
            // 1. Check if semaphore exists.
            // 2. Increment count.
            // 3. If threads are waiting, wake one up.
            todo!("Assignment 2: SemaphoreUp");
        }
        Syscall::CondVarCreate => {
            // TODO: Assignment 2 - Implement CondVar creation
            todo!("Assignment 2: CondVarCreate");
        }
        Syscall::CondVarWait { cond_id, mutex_id } => {
            // TODO: Assignment 2 - Implement CondVar Wait
            // 1. Release mutex atomically.
            // 2. Block thread on condition variable.
            // 3. Re-acquire mutex when woken.
            todo!("Assignment 2: CondVarWait");
        }
        Syscall::CondVarSignal { cond_id } => {
            // TODO: Assignment 2 - Implement CondVar Signal
            // 1. Wake one thread waiting on condition variable.
            todo!("Assignment 2: CondVarSignal");
        }
        _ => Err(TrapError::HandlerPanic("Sync: Unhandled syscall".into())),
    }
}
