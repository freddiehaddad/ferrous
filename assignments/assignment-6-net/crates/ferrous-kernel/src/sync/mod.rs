use crate::types::ThreadHandle;
use alloc::collections::VecDeque;

pub mod spinlock;
pub mod syscalls;

/// A simple Mutual Exclusion Lock.
///
/// **Assignment 2:** Students will implement `Semaphore` and `CondVar` structures here.
pub struct Mutex {
    pub id: u32,
    /// The thread currently holding the lock. `None` if unlocked.
    pub owner: Option<ThreadHandle>,
    /// Queue of threads waiting for this lock.
    pub wait_queue: VecDeque<ThreadHandle>,
}

impl Mutex {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            owner: None,
            wait_queue: VecDeque::new(),
        }
    }
}

// TODO: Add Semaphore struct here (Assignment 2)
// pub struct Semaphore { ... }

// TODO: Add CondVar struct here (Assignment 2)
// pub struct CondVar { ... }
