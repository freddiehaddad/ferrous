use core::num::NonZeroU32;

/// Thread identifier
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ThreadHandle(NonZeroU32);

impl ThreadHandle {
    pub fn new(id: u32) -> Option<Self> {
        NonZeroU32::new(id).map(Self)
    }

    pub fn val(&self) -> u32 {
        self.0.get()
    }
}
