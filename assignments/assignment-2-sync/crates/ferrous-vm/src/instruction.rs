use crate::cpu::Register;
use crate::error::DecodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // RV32I Base
    Add {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Addi {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Sub {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    And {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Or {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Xor {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Sll {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Srl {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Sra {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    // I-Type Arith
    Slti {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Sltiu {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Andi {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Ori {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Xori {
        rd: Register,
        rs1: Register,
        imm: i32,
    },
    Slli {
        rd: Register,
        rs1: Register,
        shamt: u32,
    },
    Srli {
        rd: Register,
        rs1: Register,
        shamt: u32,
    },
    Srai {
        rd: Register,
        rs1: Register,
        shamt: u32,
    },

    // R-Type Arith
    Slt {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },
    Sltu {
        rd: Register,
        rs1: Register,
        rs2: Register,
    },

    // Loads/Stores
    Lw {
        rd: Register,
        rs1: Register,
        offset: i32,
    },
    Lb {
        rd: Register,
        rs1: Register,
        offset: i32,
    },
    Lh {
        rd: Register,
        rs1: Register,
        offset: i32,
    },
    Lbu {
        rd: Register,
        rs1: Register,
        offset: i32,
    },
    Lhu {
        rd: Register,
        rs1: Register,
        offset: i32,
    },
    Sw {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Sb {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Sh {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    // Branches
    Beq {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Bne {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Blt {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Bge {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Bltu {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },
    Bgeu {
        rs1: Register,
        rs2: Register,
        offset: i32,
    },

    // Jumps
    Jal {
        rd: Register,
        offset: i32,
    },
    Jalr {
        rd: Register,
        rs1: Register,
        offset: i32,
    },

    // Upper immediate
    Lui {
        rd: Register,
        imm: u32,
    },
    Auipc {
        rd: Register,
        imm: u32,
    },

    // System
    Ecall,
    Ebreak,
}

impl Instruction {
    /// Decode a 32-bit instruction word
    pub fn decode(word: u32) -> Result<Self, DecodeError> {
        let opcode = word & 0x7F;
        let rd = ((word >> 7) & 0x1F) as u8;
        let funct3 = ((word >> 12) & 0x7) as u8;
        let rs1 = ((word >> 15) & 0x1F) as u8;
        let rs2 = ((word >> 20) & 0x1F) as u8;
        let funct7 = ((word >> 25) & 0x7F) as u8;

        let r = |n| Register::new(n).unwrap();

        match opcode {
            0x37 => {
                // LUI
                Ok(Instruction::Lui {
                    rd: r(rd),
                    imm: word & 0xFFFFF000,
                })
            }
            0x17 => {
                // AUIPC
                Ok(Instruction::Auipc {
                    rd: r(rd),
                    imm: word & 0xFFFFF000,
                })
            }
            0x6F => {
                // JAL
                let imm20 = (word >> 31) & 1;
                let imm10_1 = (word >> 21) & 0x3FF;
                let imm11 = (word >> 20) & 1;
                let imm19_12 = (word >> 12) & 0xFF;
                let mut offset = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1);
                if (offset & (1 << 20)) != 0 {
                    offset |= 0xFFE00000;
                }
                Ok(Instruction::Jal {
                    rd: r(rd),
                    offset: offset as i32,
                })
            }
            0x67 => {
                // JALR
                let imm = (word as i32) >> 20;
                Ok(Instruction::Jalr {
                    rd: r(rd),
                    rs1: r(rs1),
                    offset: imm,
                })
            }
            0x63 => {
                // BRANCH
                let imm12 = (word >> 31) & 1;
                let imm10_5 = (word >> 25) & 0x3F;
                let imm4_1 = (word >> 8) & 0xF;
                let imm11 = (word >> 7) & 1;
                let mut offset = (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1);
                if (offset & (1 << 12)) != 0 {
                    offset |= 0xFFFFE000;
                }
                let offset = offset as i32;

                match funct3 {
                    0x0 => Ok(Instruction::Beq {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x1 => Ok(Instruction::Bne {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x4 => Ok(Instruction::Blt {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x5 => Ok(Instruction::Bge {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x6 => Ok(Instruction::Bltu {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x7 => Ok(Instruction::Bgeu {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            0x03 => {
                // LOAD
                let offset = (word as i32) >> 20;
                match funct3 {
                    0x0 => Ok(Instruction::Lb {
                        rd: r(rd),
                        rs1: r(rs1),
                        offset,
                    }),
                    0x1 => Ok(Instruction::Lh {
                        rd: r(rd),
                        rs1: r(rs1),
                        offset,
                    }),
                    0x2 => Ok(Instruction::Lw {
                        rd: r(rd),
                        rs1: r(rs1),
                        offset,
                    }),
                    0x4 => Ok(Instruction::Lbu {
                        rd: r(rd),
                        rs1: r(rs1),
                        offset,
                    }),
                    0x5 => Ok(Instruction::Lhu {
                        rd: r(rd),
                        rs1: r(rs1),
                        offset,
                    }),
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            0x23 => {
                // STORE
                let imm11_5 = (word >> 25) & 0x7F;
                let imm4_0 = (word >> 7) & 0x1F;
                let mut offset = (imm11_5 << 5) | imm4_0;
                if (offset & (1 << 11)) != 0 {
                    offset |= 0xFFFFF000;
                }
                let offset = offset as i32;

                match funct3 {
                    0x0 => Ok(Instruction::Sb {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x1 => Ok(Instruction::Sh {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    0x2 => Ok(Instruction::Sw {
                        rs1: r(rs1),
                        rs2: r(rs2),
                        offset,
                    }),
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            0x13 => {
                // OP-IMM
                let imm = (word as i32) >> 20;
                let shamt = (word >> 20) & 0x1F;
                match funct3 {
                    0x0 => Ok(Instruction::Addi {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    0x1 => Ok(Instruction::Slli {
                        rd: r(rd),
                        rs1: r(rs1),
                        shamt,
                    }),
                    0x2 => Ok(Instruction::Slti {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    0x3 => Ok(Instruction::Sltiu {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    0x4 => Ok(Instruction::Xori {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    0x5 => match funct7 {
                        0x00 => Ok(Instruction::Srli {
                            rd: r(rd),
                            rs1: r(rs1),
                            shamt,
                        }),
                        0x20 => Ok(Instruction::Srai {
                            rd: r(rd),
                            rs1: r(rs1),
                            shamt,
                        }),
                        _ => Err(DecodeError::InvalidOpcode(word)),
                    },
                    0x6 => Ok(Instruction::Ori {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    0x7 => Ok(Instruction::Andi {
                        rd: r(rd),
                        rs1: r(rs1),
                        imm,
                    }),
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            0x33 => {
                // OP
                match (funct3, funct7) {
                    (0x0, 0x00) => Ok(Instruction::Add {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x0, 0x20) => Ok(Instruction::Sub {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x1, 0x00) => Ok(Instruction::Sll {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x2, 0x00) => Ok(Instruction::Slt {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x3, 0x00) => Ok(Instruction::Sltu {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x4, 0x00) => Ok(Instruction::Xor {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x5, 0x00) => Ok(Instruction::Srl {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x5, 0x20) => Ok(Instruction::Sra {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x6, 0x00) => Ok(Instruction::Or {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    (0x7, 0x00) => Ok(Instruction::And {
                        rd: r(rd),
                        rs1: r(rs1),
                        rs2: r(rs2),
                    }),
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            0x73 => {
                // SYSTEM
                match funct3 {
                    0x0 => match (word >> 20) & 0xFFF {
                        0x0 => Ok(Instruction::Ecall),
                        0x1 => Ok(Instruction::Ebreak),
                        _ => Err(DecodeError::InvalidOpcode(word)),
                    },
                    _ => Err(DecodeError::InvalidOpcode(word)),
                }
            }
            _ => Err(DecodeError::InvalidOpcode(word)),
        }
    }
}
