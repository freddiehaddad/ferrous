use crate::types::ThreadHandle;
use alloc::collections::VecDeque;

pub struct Mutex {
    pub id: u32,
    pub owner: Option<ThreadHandle>,
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
