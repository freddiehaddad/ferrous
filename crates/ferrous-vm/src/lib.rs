pub mod cpu;
pub mod devices;
pub mod error;
pub mod instruction;
pub mod memory;
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

impl VirtualMachine {
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
        let pc = PhysAddr::new(self.cpu.pc);
        let instruction_word = self.memory.read_word(pc).map_err(VmError::Memory)?;
        let instruction = Instruction::decode(instruction_word)?;

        self.cpu.pc += 4;

        match instruction {
            Instruction::Lui { rd, imm } => self.cpu.write_reg(rd, imm),
            Instruction::Auipc { rd, imm } => {
                self.cpu.write_reg(rd, pc.val().wrapping_add(imm));
            }
            Instruction::Jal { rd, offset } => {
                let target = pc.val().wrapping_add(offset as u32);
                self.cpu.write_reg(rd, pc.val() + 4);
                self.cpu.pc = target;
            }
            Instruction::Jalr { rd, rs1, offset } => {
                let base = self.cpu.read_reg(rs1);
                let target = base.wrapping_add(offset as u32) & !1;
                self.cpu.write_reg(rd, pc.val() + 4);
                self.cpu.pc = target;
            }
            Instruction::Beq { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) == self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Bne { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) != self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Blt { rs1, rs2, offset } => {
                if (self.cpu.read_reg(rs1) as i32) < (self.cpu.read_reg(rs2) as i32) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Bge { rs1, rs2, offset } => {
                if (self.cpu.read_reg(rs1) as i32) >= (self.cpu.read_reg(rs2) as i32) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Bltu { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) < self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Bgeu { rs1, rs2, offset } => {
                if self.cpu.read_reg(rs1) >= self.cpu.read_reg(rs2) {
                    self.cpu.pc = pc.val().wrapping_add(offset as u32);
                }
            }
            Instruction::Lb { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.memory.read_byte(PhysAddr::new(addr))? as i8;
                self.cpu.write_reg(rd, val as i32 as u32);
            }
            Instruction::Lh { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let b0 = self.memory.read_byte(PhysAddr::new(addr))?;
                let b1 = self.memory.read_byte(PhysAddr::new(addr + 1))?;
                let val = ((b1 as u16) << 8) | (b0 as u16);
                self.cpu.write_reg(rd, (val as i16) as i32 as u32);
            }
            Instruction::Lw { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.memory.read_word(PhysAddr::new(addr))?;
                self.cpu.write_reg(rd, val);
            }
            Instruction::Lbu { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.memory.read_byte(PhysAddr::new(addr))?;
                self.cpu.write_reg(rd, val as u32);
            }
            Instruction::Lhu { rd, rs1, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let b0 = self.memory.read_byte(PhysAddr::new(addr))?;
                let b1 = self.memory.read_byte(PhysAddr::new(addr + 1))?;
                let val = ((b1 as u16) << 8) | (b0 as u16);
                self.cpu.write_reg(rd, val as u32);
            }
            Instruction::Sb { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.cpu.read_reg(rs2) as u8;
                self.memory.write_byte(PhysAddr::new(addr), val)?;
            }
            Instruction::Sh { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.cpu.read_reg(rs2) as u16;
                self.memory.write_byte(PhysAddr::new(addr), val as u8)?;
                self.memory
                    .write_byte(PhysAddr::new(addr + 1), (val >> 8) as u8)?;
            }
            Instruction::Sw { rs1, rs2, offset } => {
                let addr = self.cpu.read_reg(rs1).wrapping_add(offset as u32);
                let val = self.cpu.read_reg(rs2);
                self.memory.write_word(PhysAddr::new(addr), val)?;
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
                self.cpu.pc = pc.val(); // Rewind PC for trap handler
                let cause = match self.cpu.mode {
                    PrivilegeMode::User => TrapCause::EnvironmentCallFromU,
                    PrivilegeMode::Supervisor => TrapCause::EnvironmentCallFromS,
                    PrivilegeMode::Machine => TrapCause::EnvironmentCallFromS,
                };
                return Ok(StepResult::Trap(cause));
            }
            Instruction::Ebreak => {
                self.cpu.pc = pc.val();
                return Ok(StepResult::Trap(TrapCause::Breakpoint));
            }
        }

        Ok(StepResult::Continue)
    }
}
