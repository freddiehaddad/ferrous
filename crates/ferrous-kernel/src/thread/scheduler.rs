use crate::types::ThreadHandle;
use alloc::collections::VecDeque;

pub trait Scheduler: Send {
    /// Select next thread to run
    fn schedule(&mut self) -> Option<ThreadHandle>;

    /// Add thread to ready queue
    fn enqueue(&mut self, thread: ThreadHandle);

    /// Remove thread from ready queue (e.g. if blocked or terminated)
    fn dequeue(&mut self, thread: ThreadHandle) -> bool;

    /// Called on timer tick
    fn tick(&mut self);
}

pub struct RoundRobinScheduler {
    ready_queue: VecDeque<ThreadHandle>,
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
        self.ready_queue.pop_front()
    }

    fn enqueue(&mut self, thread: ThreadHandle) {
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
    }
}
