use crate::syscall;

pub struct Mutex {
    id: u32,
}

impl Default for Mutex {
    fn default() -> Self {
        Self::new()
    }
}

impl Mutex {
    pub fn new() -> Self {
        let id = syscall::mutex_create();
        Self { id }
    }

    pub fn lock(&self) {
        syscall::mutex_acquire(self.id);
    }

    pub fn unlock(&self) {
        syscall::mutex_release(self.id);
    }
}
