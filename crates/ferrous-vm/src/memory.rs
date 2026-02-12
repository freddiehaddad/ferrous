use bytemuck::{Pod, Zeroable};
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Physical memory address (cannot be dereferenced directly)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct PhysAddr(pub u32);

impl PhysAddr {
    pub const fn new(addr: u32) -> Self {
        Self(addr)
    }

    pub const fn val(&self) -> u32 {
        self.0
    }
}

impl Add<u32> for PhysAddr {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for PhysAddr {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

impl Sub<u32> for PhysAddr {
    type Output = Self;
    fn sub(self, rhs: u32) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u32> for PhysAddr {
    fn sub_assign(&mut self, rhs: u32) {
        self.0 -= rhs;
    }
}

/// Virtual memory address
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Pod, Zeroable)]
#[repr(transparent)]
pub struct VirtAddr(pub u32);

impl VirtAddr {
    pub const fn new(addr: u32) -> Self {
        Self(addr)
    }

    pub const fn val(&self) -> u32 {
        self.0
    }
}

impl Add<u32> for VirtAddr {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for VirtAddr {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

impl Sub<u32> for VirtAddr {
    type Output = Self;
    fn sub(self, rhs: u32) -> Self {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u32> for VirtAddr {
    fn sub_assign(&mut self, rhs: u32) {
        self.0 -= rhs;
    }
}

/// Page number (physical)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PhysPageNum(pub u32);

/// Page number (virtual)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VirtPageNum(pub u32);

/// Memory Access trait
pub trait Memory {
    fn read_byte(&self, addr: PhysAddr) -> Result<u8, crate::error::MemoryError>;
    fn write_byte(&mut self, addr: PhysAddr, val: u8) -> Result<(), crate::error::MemoryError>;

    fn read_word(&self, addr: PhysAddr) -> Result<u32, crate::error::MemoryError> {
        let b0 = self.read_byte(addr)? as u32;
        let b1 = self.read_byte(addr + 1)? as u32;
        let b2 = self.read_byte(addr + 2)? as u32;
        let b3 = self.read_byte(addr + 3)? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    fn write_word(&mut self, addr: PhysAddr, val: u32) -> Result<(), crate::error::MemoryError> {
        self.write_byte(addr, (val & 0xFF) as u8)?;
        self.write_byte(addr + 1, ((val >> 8) & 0xFF) as u8)?;
        self.write_byte(addr + 2, ((val >> 16) & 0xFF) as u8)?;
        self.write_byte(addr + 3, ((val >> 24) & 0xFF) as u8)?;
        Ok(())
    }
}

pub struct SimpleMemory {
    data: Vec<u8>,
    base_addr: u32,
}

impl SimpleMemory {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
            base_addr: 0x8000_0000,
        }
    }

    pub fn load(&mut self, addr: PhysAddr, data: &[u8]) -> Result<(), crate::error::MemoryError> {
        if addr.0 < self.base_addr {
            return Err(crate::error::MemoryError::OutOfBounds(addr.0));
        }
        let start = (addr.0 - self.base_addr) as usize;
        let end = start + data.len();
        if end > self.data.len() {
            return Err(crate::error::MemoryError::OutOfBounds(end as u32));
        }
        self.data[start..end].copy_from_slice(data);
        Ok(())
    }
}

impl Memory for SimpleMemory {
    fn read_byte(&self, addr: PhysAddr) -> Result<u8, crate::error::MemoryError> {
        if addr.0 >= self.base_addr {
            let offset = (addr.0 - self.base_addr) as usize;
            if offset < self.data.len() {
                return Ok(self.data[offset]);
            }
        }
        Err(crate::error::MemoryError::OutOfBounds(addr.0))
    }

    fn write_byte(&mut self, addr: PhysAddr, val: u8) -> Result<(), crate::error::MemoryError> {
        if addr.0 >= self.base_addr {
            let offset = (addr.0 - self.base_addr) as usize;
            if offset < self.data.len() {
                self.data[offset] = val;
                return Ok(());
            }
        }
        Err(crate::error::MemoryError::OutOfBounds(addr.0))
    }
}
