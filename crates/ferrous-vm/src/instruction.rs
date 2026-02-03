use crate::cpu::Register;
use crate::error::DecodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // ========================================================================
    // RV32I Base Integer Instruction Set (47 instructions)
    // ========================================================================

    // ------------------------------------------------------------------------
    // Integer Computational Instructions - Register-Immediate (I-type)
    // ------------------------------------------------------------------------
    /// Add immediate: rd = rs1 + imm
    Addi {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// Set less than immediate (signed): rd = (rs1 < imm) ? 1 : 0
    Slti {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// Set less than immediate unsigned: rd = (rs1 < imm) ? 1 : 0 (unsigned comparison)
    Sltiu {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// XOR immediate: rd = rs1 ^ imm
    Xori {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// OR immediate: rd = rs1 | imm
    Ori {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// AND immediate: rd = rs1 & imm
    Andi {
        rd: Register,
        rs1: Register,
        imm: i32,
    },

    /// Shift left logical immediate: rd = rs1 << shamt
    Slli {
        rd: Register,
        rs1: Register,
        shamt: u32, // 5-bit shift amount (0-31)
    },

    /// Shift right logical immediate: rd = rs1 >> shamt (zero-extend)
    Srli {
        rd: Register,
        rs1: Register,
        shamt: u32,
    },

    /// Shift right arithmetic immediate: rd = rs1 >> shamt (sign-extend)
    Srai {
        rd: Register,
        rs1: Register,
        shamt: u32,
    },

    // ------------------------------------------------------------------------
    // Integer Computational Instructions - Register-Register (R-type)
    // ------------------------------------------------------------------------
    /// Add: rd = rs1 + rs2
    Add {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Subtract: rd = rs1 - rs2
    Sub {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Shift left logical: rd = rs1 << rs2
    Sll {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Set less than (signed): rd = (rs1 < rs2) ? 1 : 0
    Slt {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Set less than unsigned: rd = (rs1 < rs2) ? 1 : 0 (unsigned comparison)
    Sltu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// XOR: rd = rs1 ^ rs2
    Xor {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Shift right logical: rd = rs1 >> rs2 (zero-extend)
    Srl {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Shift right arithmetic: rd = rs1 >> rs2 (sign-extend)
    Sra {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// OR: rd = rs1 | rs2
    Or {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// AND: rd = rs1 & rs2
    And {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    // ------------------------------------------------------------------------
    // Load Instructions (I-type)
    // ------------------------------------------------------------------------
    /// Load byte (sign-extended): rd = sign_extend(mem[rs1 + offset][7:0])
    Lb {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    /// Load halfword (sign-extended): rd = sign_extend(mem[rs1 + offset][15:0])
    Lh {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    /// Load word: rd = mem[rs1 + offset][31:0]
    Lw {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    /// Load byte unsigned (zero-extended): rd = zero_extend(mem[rs1 + offset][7:0])
    Lbu {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    /// Load halfword unsigned (zero-extended): rd = zero_extend(mem[rs1 + offset][15:0])
    Lhu {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    // ------------------------------------------------------------------------
    // Store Instructions (S-type)
    // ------------------------------------------------------------------------
    /// Store byte: mem[rs1 + offset][7:0] = rs2[7:0]
    Sb {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Store halfword: mem[rs1 + offset][15:0] = rs2[15:0]
    Sh {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Store word: mem[rs1 + offset][31:0] = rs2[31:0]
    Sw {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    // ------------------------------------------------------------------------
    // Branch Instructions (B-type)
    // ------------------------------------------------------------------------
    /// Branch if equal: if (rs1 == rs2) pc += offset
    Beq {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Branch if not equal: if (rs1 != rs2) pc += offset
    Bne {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Branch if less than (signed): if (rs1 < rs2) pc += offset
    Blt {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Branch if greater or equal (signed): if (rs1 >= rs2) pc += offset
    Bge {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Branch if less than unsigned: if (rs1 < rs2) pc += offset (unsigned)
    Bltu {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    /// Branch if greater or equal unsigned: if (rs1 >= rs2) pc += offset (unsigned)
    Bgeu {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    // ------------------------------------------------------------------------
    // Jump Instructions
    // ------------------------------------------------------------------------
    /// Jump and link: rd = pc + 4; pc += offset
    Jal { rd: Register, offset: i32 },

    /// Jump and link register: rd = pc + 4; pc = (rs1 + offset) & ~1
    Jalr {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    // ------------------------------------------------------------------------
    // Upper Immediate Instructions (U-type)
    // ------------------------------------------------------------------------
    /// Load upper immediate: rd = imm << 12
    Lui {
        rd: Register,
        imm: u32, // 20-bit immediate
    },

    /// Add upper immediate to PC: rd = pc + (imm << 12)
    Auipc {
        rd: Register,
        imm: u32, // 20-bit immediate
    },

    // ------------------------------------------------------------------------
    // System Instructions
    // ------------------------------------------------------------------------
    /// Environment call (system call)
    Ecall,

    /// Environment breakpoint (debugger breakpoint)
    Ebreak,

    // ------------------------------------------------------------------------
    // Memory Ordering Instructions
    // ------------------------------------------------------------------------
    /// Fence memory and I/O operations
    /// pred: predecessor operations (bits: PI, PO, PR, PW)
    /// succ: successor operations (bits: SI, SO, SR, SW)
    Fence {
        pred: u8, // 4-bit predecessor set
        succ: u8, // 4-bit successor set
    },

    // ------------------------------------------------------------------------
    // Control and Status Register (CSR) Instructions
    // ------------------------------------------------------------------------
    /// CSR read/write: rd = CSR[csr]; CSR[csr] = rs1
    Csrrw {
        rd: Register,
        csr: u16, // 12-bit CSR address
        rs1: Register,
    },

    /// CSR read and set bits: rd = CSR[csr]; CSR[csr] = CSR[csr] | rs1
    Csrrs {
        rd: Register,
        csr: u16,
        rs1: Register,
    },

    /// CSR read and clear bits: rd = CSR[csr]; CSR[csr] = CSR[csr] & ~rs1
    Csrrc {
        rd: Register,
        csr: u16,
        rs1: Register,
    },

    /// CSR read/write immediate: rd = CSR[csr]; CSR[csr] = imm
    Csrrwi {
        rd: Register,
        csr: u16,
        imm: u32, // 5-bit unsigned immediate (zero-extended)
    },

    /// CSR read and set bits immediate: rd = CSR[csr]; CSR[csr] = CSR[csr] | imm
    Csrrsi { rd: Register, csr: u16, imm: u32 },

    /// CSR read and clear bits immediate: rd = CSR[csr]; CSR[csr] = CSR[csr] & ~imm
    Csrrci { rd: Register, csr: u16, imm: u32 },

    // ========================================================================
    // RV32M Standard Extension (8 instructions)
    // Integer Multiplication and Division
    // ========================================================================
    /// Multiply: rd = (rs1 * rs2)[31:0] (lower 32 bits)
    Mul {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Multiply high signed×signed: rd = (rs1 * rs2)[63:32] (upper 32 bits)
    Mulh {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Multiply high signed×unsigned: rd = (rs1 * rs2)[63:32]
    Mulhsu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Multiply high unsigned×unsigned: rd = (rs1 * rs2)[63:32]
    Mulhu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Divide signed: rd = rs1 / rs2 (rounded toward zero)
    Div {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Divide unsigned: rd = rs1 / rs2 (rounded toward zero)
    Divu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Remainder signed: rd = rs1 % rs2
    Rem {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Remainder unsigned: rd = rs1 % rs2
    Remu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    // ========================================================================
    // RV32A Standard Extension (11 instructions)
    // Atomic Instructions
    // ========================================================================
    /// Load-reserved word: rd = mem[rs1]; reserve(rs1)
    LrW { rd: Register, rs1: Register },

    /// Store-conditional word: if (reservation valid) { mem[rs1] = rs2; rd = 0 } else { rd = 1 }
    ScW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic swap word: rd = mem[rs1]; mem[rs1] = rs2
    AmoSwapW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic add word: rd = mem[rs1]; mem[rs1] = mem[rs1] + rs2
    AmoAddW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic XOR word: rd = mem[rs1]; mem[rs1] = mem[rs1] ^ rs2
    AmoXorW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic AND word: rd = mem[rs1]; mem[rs1] = mem[rs1] & rs2
    AmoAndW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic OR word: rd = mem[rs1]; mem[rs1] = mem[rs1] | rs2
    AmoOrW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic minimum word (signed): rd = mem[rs1]; mem[rs1] = min(mem[rs1], rs2)
    AmoMinW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic maximum word (signed): rd = mem[rs1]; mem[rs1] = max(mem[rs1], rs2)
    AmoMaxW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic minimum word unsigned: rd = mem[rs1]; mem[rs1] = min(mem[rs1], rs2)
    AmoMinuW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    /// Atomic maximum word unsigned: rd = mem[rs1]; mem[rs1] = max(mem[rs1], rs2)
    AmoMaxuW {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
}

impl Instruction {
    /// Decode a 32-bit instruction word into an Instruction variant
    ///
    /// # Arguments
    ///
    /// * `word` - The 32-bit RISC-V instruction encoding
    ///
    /// # Returns
    ///
    /// * `Ok(Instruction)` - Successfully decoded instruction
    /// * `Err(DecodeError)` - Invalid opcode or encoding
    ///
    /// # Examples
    ///
    /// ```
    /// use ferrous_vm::instruction::Instruction;
    ///
    /// // Decode ADDI x1, x0, 42
    /// let word = 0x02a00093;
    /// let inst = Instruction::decode(word).unwrap();
    /// ```
    pub fn decode(word: u32) -> Result<Self, DecodeError> {
        let opcode = word & 0x7F;

        match opcode {
            _ => todo!("Implement instruction decoder (see RISC-V spec for encoding details)"),
        }
    }
}
