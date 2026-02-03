/// Physical memory address (cannot be dereferenced directly)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhysAddr(u32);

/// Virtual memory address
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct VirtAddr(u32);

/// Page number (physical)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PhysPageNum(u32);

/// Page number (virtual)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct VirtPageNum(u32);
