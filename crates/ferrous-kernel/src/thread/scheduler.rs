use crate::types::ThreadHandle;
use alloc::collections::VecDeque;

/// The `Scheduler` trait defines the interface for thread scheduling algorithms.
/// Different implementations (e.g., Round Robin, Priority, MLFQ) can be swapped in.
pub trait Scheduler: Send {
    /// Select the next thread to run from the ready queue.
    /// Returns `None` if no threads are ready.
    fn schedule(&mut self) -> Option<ThreadHandle>;

    /// Add a thread to the ready queue.
    /// This is called when a thread is created or transitions from Blocked -> Ready.
    fn enqueue(&mut self, thread: ThreadHandle);

    /// Remove a thread from the ready queue.
    /// This is called when a thread is blocked or terminated.
    /// Returns `true` if the thread was found and removed.
    fn dequeue(&mut self, thread: ThreadHandle) -> bool;

    /// Called on every timer interrupt (typically 10ms or 100ms).
    /// Used for preemptive scheduling logic (e.g., time slicing).
    fn tick(&mut self);
}

/// A simple Round-Robin Scheduler.
///
/// **Assignment 1:** Students will implement this scheduler.
/// **Assignment 3:** Students will implement `PriorityScheduler` and `MlfqScheduler`.
pub struct RoundRobinScheduler {
    ready_queue: VecDeque<ThreadHandle>,
    // TODO: Add time quantum tracking here for preemption (Assignment 3)
}

impl Default for RoundRobinScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn schedule(&mut self) -> Option<ThreadHandle> {
        // Pop the first thread from the queue (FIFO)
        self.ready_queue.pop_front()
    }

    fn enqueue(&mut self, thread: ThreadHandle) {
        // Add the thread to the back of the queue
        self.ready_queue.push_back(thread);
    }

    fn dequeue(&mut self, thread: ThreadHandle) -> bool {
        // Not efficient for deque but correct
        if let Some(pos) = self.ready_queue.iter().position(|&h| h == thread) {
            self.ready_queue.remove(pos);
            true
        } else {
            false
        }
    }

    fn tick(&mut self) {
        // Round robin usually rotates on tick if time slice expired
        // For now simple FIFO until we add preemption logic
        // TODO: Implement time slicing logic here (Assignment 3)
    }
}
