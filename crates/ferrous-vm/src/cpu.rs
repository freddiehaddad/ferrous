/// RISC-V register number (x0-x31)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Register(u8);

impl Register {
    // Special registers

    /// x0: Hard-wired zero register
    pub const ZERO: Register = Register(0);

    /// x1: Return address
    pub const RA: Register = Register(1);

    /// x2: Stack pointer
    pub const SP: Register = Register(2);

    /// x3: Global pointer
    pub const GP: Register = Register(3);

    /// x4: Thread pointer
    pub const TP: Register = Register(4);

    // Temporary registers (caller-saved)

    /// x5: Temporary register 0 / Alternate link register
    pub const T0: Register = Register(5);

    /// x6: Temporary register 1
    pub const T1: Register = Register(6);

    /// x7: Temporary register 2
    pub const T2: Register = Register(7);

    // Saved registers (callee-saved)

    /// x8: Saved register 0 / Frame pointer
    pub const S0: Register = Register(8);
    /// x8: Frame pointer (alias for S0)
    pub const FP: Register = Register(8);

    /// x9: Saved register 1
    pub const S1: Register = Register(9);

    // Function arguments / return values

    /// x10: Function argument 0 / Return value 0
    pub const A0: Register = Register(10);

    /// x11: Function argument 1 / Return value 1
    pub const A1: Register = Register(11);

    /// x12: Function argument 2
    pub const A2: Register = Register(12);

    /// x13: Function argument 3
    pub const A3: Register = Register(13);

    /// x14: Function argument 4
    pub const A4: Register = Register(14);

    /// x15: Function argument 5
    pub const A5: Register = Register(15);

    /// x16: Function argument 6
    pub const A6: Register = Register(16);

    /// x17: Function argument 7
    pub const A7: Register = Register(17);

    // More saved registers (callee-saved)

    /// x18: Saved register 2
    pub const S2: Register = Register(18);

    /// x19: Saved register 3
    pub const S3: Register = Register(19);

    /// x20: Saved register 4
    pub const S4: Register = Register(20);

    /// x21: Saved register 5
    pub const S5: Register = Register(21);

    /// x22: Saved register 6
    pub const S6: Register = Register(22);

    /// x23: Saved register 7
    pub const S7: Register = Register(23);

    /// x24: Saved register 8
    pub const S8: Register = Register(24);

    /// x25: Saved register 9
    pub const S9: Register = Register(25);

    /// x26: Saved register 10
    pub const S10: Register = Register(26);

    /// x27: Saved register 11
    pub const S11: Register = Register(27);

    // More temporary registers (caller-saved)

    /// x28: Temporary register 3
    pub const T3: Register = Register(28);

    /// x29: Temporary register 4
    pub const T4: Register = Register(29);

    /// x30: Temporary register 5
    pub const T5: Register = Register(30);

    /// x31: Temporary register 6
    pub const T6: Register = Register(31);

    /// Create a new register from a register number (0-31)
    ///
    /// # Errors
    ///
    /// Returns `InvalidRegister` if `num >= 32`
    ///
    /// # Examples
    ///
    ///    /// use ferrous_vm::cpu::Register;
    ///
    /// let sp = Register::new(2).unwrap();
    /// assert_eq!(sp, Register::SP);
    ///
    /// let invalid = Register::new(32);
    /// asser
    pub fn new(num: u8) -> Result<Self, InvalidRegister> {
        if num < 32 {
            Ok(Register(num))
        } else {
            Err(InvalidRegister(num))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid register number: {0} (must be 0-31)")]
pub struct InvalidRegister(pub u8);
