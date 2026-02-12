#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Register(u8);

impl Register {
    pub const ZERO: Register = Register(0); // x0
    pub const RA: Register = Register(1); // x1 (return address)
    pub const SP: Register = Register(2); // x2 (stack pointer)
                                          // ... we can add constants for all registers if needed, but for now just common ones

    pub fn new(num: u8) -> Result<Self, crate::error::DecodeError> {
        if num < 32 {
            Ok(Register(num))
        } else {
            // Technically this should be a decode error or similar
            Err(crate::error::DecodeError::InvalidEncoding(num as u32))
        }
    }

    pub fn val(&self) -> usize {
        self.0 as usize
    }
}

pub struct Cpu {
    pub pc: u32,
    pub regs: [u32; 32],
    pub mode: PrivilegeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeMode {
    User,
    Supervisor,
    Machine, // Adding Machine mode as it is often the root mode in RISC-V, even if not fully used yet
}

impl Cpu {
    pub fn new(entry_point: u32) -> Self {
        Self {
            pc: entry_point,
            regs: [0; 32],
            mode: PrivilegeMode::Machine,
        }
    }

    pub fn read_reg(&self, reg: Register) -> u32 {
        if reg == Register::ZERO {
            0
        } else {
            self.regs[reg.val()]
        }
    }

    pub fn write_reg(&mut self, reg: Register, val: u32) {
        if reg != Register::ZERO {
            self.regs[reg.val()] = val;
        }
    }
}
