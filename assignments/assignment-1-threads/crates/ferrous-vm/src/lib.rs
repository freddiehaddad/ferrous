#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;

pub mod cpu;
pub mod devices;
pub mod error;
pub mod instruction;
pub mod memory;
pub mod mmu;
pub mod system_bus;
pub mod trap;

pub use cpu::*;
pub use devices::*;
pub use error::*;
pub use instruction::*;
pub use memory::*;
pub use trap::*;

pub struct VmConfig {
    pub memory_size: usize,
    pub timer_interval: Option<u64>,
}

pub struct VirtualMachine {
    pub cpu: Cpu,
    pub memory: Box<dyn Memory>,
    pub trap_handler: Box<dyn TrapHandler>,
    pub config: VmConfig,
    pub instruction_count: u64,
    pub next_timer_interrupt: u64,
}

#[derive(Debug, PartialEq)]
pub enum ExitReason {
    Halt,
    Breakpoint,
    Error(VmError),
}

#[derive(Debug)]
pub enum StepResult {
    Continue,
    Trap(TrapCause),
    Exit(ExitReason),
}

use mmu::AccessType;

impl VirtualMachine {
    fn translate(&mut self, addr: VirtAddr, access: AccessType) -> Result<PhysAddr, TrapCause> {
        mmu::translate(
            addr,
            access,
            self.cpu.satp,
            self.cpu.mode,
            self.memory.as_mut(),
        )
    }

    pub fn new(
        config: VmConfig,
        memory: Box<dyn Memory>,
        trap_handler: Box<dyn TrapHandler>,
    ) -> Result<Self, VmError> {
        Ok(Self {
            cpu: Cpu::new(0x8000_0000), // Standard entry point
            memory,
            trap_handler,
            instruction_count: 0,
            next_timer_interrupt: config.timer_interval.unwrap_or(u64::MAX),
            config,
        })
    }

    pub fn load_program(&mut self, binary: &[u8], entry_point: VirtAddr) -> Result<(), VmError> {
        let phys_addr = PhysAddr::new(entry_point.val());
        for (i, &byte) in binary.iter().enumerate() {
            self.memory.write_byte(phys_addr + (i as u32), byte)?;
        }
        self.cpu.pc = entry_point.val();
        Ok(())
    }

    pub fn run(&mut self) -> Result<ExitReason, VmError> {
        loop {
            // Check for timer interrupt
            if let Some(interval) = self.config.timer_interval {
                if self.instruction_count >= self.next_timer_interrupt {
                    self.next_timer_interrupt += interval;
                    let result = self.trap_handler.handle_trap(
                        TrapCause::TimerInterrupt,
                        &mut self.cpu,
                        self.memory.as_mut(),
                    );
                    match result {
                        Ok(resume_addr) => self.cpu.pc = resume_addr.val(),
                        Err(TrapError::Halt) => return Ok(ExitReason::Halt),
                        Err(e) => return Err(VmError::Trap(e)),
                    }
                }
            }

            let step_result = self.step();
            self.instruction_count += 1;

            match step_result {
                Ok(StepResult::Continue) => {
                    continue;
                }
                Ok(StepResult::Exit(reason)) => return Ok(reason),
                Ok(StepResult::Trap(cause)) => {
                    let result =
                        self.trap_handler
                            .handle_trap(cause, &mut self.cpu, self.memory.as_mut());
                    match result {
                        Ok(resume_addr) => self.cpu.pc = resume_addr.val(),
                        Err(TrapError::Halt) => return Ok(ExitReason::Halt),
                        Err(e) => return Err(VmError::Trap(e)),
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn step(&mut self) -> Result<StepResult, VmError> {
        // Fetch
        let pc_val = self.cpu.pc;
        let pc_virt = VirtAddr::new(pc_val);
        let pc_phys = match self.translate(pc_virt, AccessType::Execute) {
            Ok(pa) => pa,
            Err(e) => return Ok(StepResult::Trap(e)),
        };

        let instruction_word = self.memory.read_word(pc_phys).map_err(VmError::Memory)?;
        let instruction = Instruction::decode(instruction_word)?;

        self.cpu.pc += 4;

        // Helper macro for data translation
        macro_rules! translate_data {
            ($addr:expr, $access:expr) => {
                match self.translate(VirtAddr::new($addr), $access) {
                    Ok(pa) => pa,
                    Err(e) => return Ok(StepResult::Trap(e)),
                }
            };
        }

        match instruction {
            Instruction::Lui { rd, imm } => self.cpu.write_reg(rd, imm),
            Instruction::Auipc { rd, imm } => {
                self.cpu.write_reg(rd, pc_val.wrapping_add(imm));
            }
            Instruction::Jal { rd, offset } => {
                let target = pc_val.wrapping_add(offset as u32);
                self.cpu.write_reg(rd, pc_val + 4);
                self.cpu.pc = target;
            }
            Instruction::Jalr { rd, rs1, offset } => {
                let base = self.cpu.read_reg(rs1);
                let target = base.wrapping_add(offset as u32) & !1;
                self.cpu.write_reg(rd, pc_val + 4);
                self.cpu.pc = target;
            }
            Instruction::Beq { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) == self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Bne { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) != self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Blt { rs1, rs2, offset } => {
                if (self.cpu.read_reg(rs1) as i32) < (self.cpu.read_reg(rs2) as i32) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Bge { rs1, rs2, offset } => {
                if (self.cpu.read_reg(rs1) as i32) >= (self.cpu.read_reg(rs2) as i32) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Bltu { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) < self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Bgeu { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) >= self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc_val.wrapping_add(offset as u32);
                }
            }
            Instruction::Lb { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Read);
                let val = self.memory.read_byte(phys).map_err(VmError::Memory)? as i8;
                self.cpu.write_reg(rd, val as i32 as u32);
            }
            Instruction::Lh { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Read);
                // Note: read_word/read_u16 logic handles splitting, but we need to ensure checks?
                // For simplicity, we assume pages are 4-byte aligned or handle split pages later.
                // MMU translate returns PA for the start byte.
                // If it crosses a page boundary, we might have issues if we just add +1 to PA.
                // Real hardware checks both pages.
                // For now, let's assume simple implementation:
                let b0 = self.memory.read_byte(phys).map_err(VmError::Memory)?;
                // Check next byte address translation again?
                // Technically required for page crossing.
                // For this iteration, let's assume contiguous PA if not crossing page boundary.
                // Or just translate every byte? That's slow.
                // Let's rely on standard check:
                if (addr & 0xFFF) == 0xFFF {
                    // Crosses page boundary
                    let phys2 = translate_data!(addr + 1, AccessType::Read);
                    let b1 = self.memory.read_byte(phys2).map_err(VmError::Memory)?;
                    let val = ((b1 as u16) << 8) | (b0 as u16);
                    self.cpu.write_reg(rd, (val as i16) as i32 as u32);
                } else {
                    let b1 = self.memory.read_byte(phys + 1).map_err(VmError::Memory)?;
                    let val = ((b1 as u16) << 8) | (b0 as u16);
                    self.cpu.write_reg(rd, (val as i16) as i32 as u32);
                }
            }
            Instruction::Lw { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Read);
                // Check page boundary crossing for 4 bytes
                if (addr & 0xFFF) > 0xFFC {
                    // Slow path: read byte by byte with translation
                    let mut bytes = [0u8; 4];
                    for i in 0..4 {
                        let pa = translate_data!(addr + i, AccessType::Read);
                        bytes[i as usize] = self.memory.read_byte(pa).map_err(VmError::Memory)?;
                    }
                    let val = u32::from_le_bytes(bytes);
                    self.cpu.write_reg(rd, val);
                } else {
                    let val = self.memory.read_word(phys).map_err(VmError::Memory)?;
                    self.cpu.write_reg(rd, val);
                }
            }
            Instruction::Lbu { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Read);
                let val = self.memory.read_byte(phys).map_err(VmError::Memory)?;
                self.cpu.write_reg(rd, val as u32);
            }
            Instruction::Lhu { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Read);
                if (addr & 0xFFF) == 0xFFF {
                    let phys2 = translate_data!(addr + 1, AccessType::Read);
                    let b0 = self.memory.read_byte(phys).map_err(VmError::Memory)?;
                    let b1 = self.memory.read_byte(phys2).map_err(VmError::Memory)?;
                    let val = ((b1 as u16) << 8) | (b0 as u16);
                    self.cpu.write_reg(rd, val as u32);
                } else {
                    let b0 = self.memory.read_byte(phys).map_err(VmError::Memory)?;
                    let b1 = self.memory.read_byte(phys + 1).map_err(VmError::Memory)?;
                    let val = ((b1 as u16) << 8) | (b0 as u16);
                    self.cpu.write_reg(rd, val as u32);
                }
            }
            Instruction::Sb { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let phys = translate_data!(addr, AccessType::Write);
                let val = self.cpu.read_reg(rs2) as u8;
                self.memory.write_byte(phys, val).map_err(VmError::Memory)?;
            }
            Instruction::Sh { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.cpu.read_reg(rs2) as u16;
                let phys = translate_data!(addr, AccessType::Write);

                if (addr & 0xFFF) == 0xFFF {
                    let phys2 = translate_data!(addr + 1, AccessType::Write);
                    self.memory
                        .write_byte(phys, val as u8)
                        .map_err(VmError::Memory)?;
                    self.memory
                        .write_byte(phys2, (val >> 8) as u8)
                        .map_err(VmError::Memory)?;
                } else {
                    self.memory
                        .write_byte(phys, val as u8)
                        .map_err(VmError::Memory)?;
                    self.memory
                        .write_byte(phys + 1, (val >> 8) as u8)
                        .map_err(VmError::Memory)?;
                }
            }
            Instruction::Sw { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.cpu.read_reg(rs2);
                let phys = translate_data!(addr, AccessType::Write);

                if (addr & 0xFFF) > 0xFFC {
                    let bytes = val.to_le_bytes();
                    for i in 0..4 {
                        let pa = translate_data!(addr + i, AccessType::Write);
                        self.memory
                            .write_byte(pa, bytes[i as usize])
                            .map_err(VmError::Memory)?;
                    }
                } else {
                    self.memory.write_word(phys, val).map_err(VmError::Memory)?;
                }
            }
            Instruction::Addi { rd, rs1, imm } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1).wrapping_add(imm as u32));
            }
            Instruction::Slti { rd, rs1, imm } => {
                let val = if (self.cpu.read_reg(rs1) as i32) < imm {
                    1
                } else {
                    0
                };
                self.cpu.write_reg(rd, val);
            }
            Instruction::Sltiu { rd, rs1, imm } => {
                let val = if self.cpu.read_reg(rs1) < (imm as u32) {
                    1
                } else {
                    0
                };
                self.cpu.write_reg(rd, val);
            }
            Instruction::Xori { rd, rs1, imm } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) ^ (imm as u32));
            }
            Instruction::Ori { rd, rs1, imm } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) | (imm as u32));
            }
            Instruction::Andi { rd, rs1, imm } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) & (imm as u32));
            }
            Instruction::Slli { rd, rs1, shamt } => {
                self.cpu.write_reg(rd, self.cpu.read_reg(rs1) << shamt);
            }
            Instruction::Srli { rd, rs1, shamt } => {
                self.cpu.write_reg(rd, self.cpu.read_reg(rs1) >> shamt);
            }
            Instruction::Srai { rd, rs1, shamt } => {
                self.cpu
                    .write_reg(rd, ((self.cpu.read_reg(rs1) as i32) >> shamt) as u32);
            }
            Instruction::Add { rd, rs1, rs2 } => {
                self.cpu.write_reg(
                    rd,
                    self.cpu.read_reg(rs1).wrapping_add(self.cpu.read_reg(rs2)),
                );
            }
            Instruction::Sub { rd, rs1, rs2 } => {
                self.cpu.write_reg(
                    rd,
                    self.cpu.read_reg(rs1).wrapping_sub(self.cpu.read_reg(rs2)),
                );
            }
            Instruction::Sll { rd, rs1, rs2 } => {
                let shamt = self.cpu.read_reg(rs2) & 0x1F;
                self.cpu.write_reg(rd, self.cpu.read_reg(rs1) << shamt);
            }
            Instruction::Slt { rd, rs1, rs2 } => {
                let val = if (self.cpu.read_reg(rs1) as i32) < (self.cpu.read_reg(rs2) as i32) {
                    1
                } else {
                    0
                };
                self.cpu.write_reg(rd, val);
            }
            Instruction::Sltu { rd, rs1, rs2 } => {
                let val = if self.cpu.read_reg(rs1) < self.cpu.read_reg(rs2) {
                    1
                } else {
                    0
                };
                self.cpu.write_reg(rd, val);
            }
            Instruction::Xor { rd, rs1, rs2 } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) ^ self.cpu.read_reg(rs2));
            }
            Instruction::Srl { rd, rs1, rs2 } => {
                let shamt = self.cpu.read_reg(rs2) & 0x1F;
                self.cpu.write_reg(rd, self.cpu.read_reg(rs1) >> shamt);
            }
            Instruction::Sra { rd, rs1, rs2 } => {
                let shamt = self.cpu.read_reg(rs2) & 0x1F;
                self.cpu
                    .write_reg(rd, ((self.cpu.read_reg(rs1) as i32) >> shamt) as u32);
            }
            Instruction::Or { rd, rs1, rs2 } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) | self.cpu.read_reg(rs2));
            }
            Instruction::And { rd, rs1, rs2 } => {
                self.cpu
                    .write_reg(rd, self.cpu.read_reg(rs1) & self.cpu.read_reg(rs2));
            }
            Instruction::Ecall => {
                self.cpu.pc = pc_val; // Rewind PC for trap handler
                let cause = match self.cpu.mode {
                    PrivilegeMode::User => TrapCause::EnvironmentCallFromU,
                    PrivilegeMode::Supervisor => TrapCause::EnvironmentCallFromS,
                    PrivilegeMode::Machine => TrapCause::EnvironmentCallFromS,
                };
                return Ok(StepResult::Trap(cause));
            }
            Instruction::Ebreak => {
                self.cpu.pc = pc_val;
                return Ok(StepResult::Trap(TrapCause::Breakpoint));
            }
        }

        Ok(StepResult::Continue)
    }
}
