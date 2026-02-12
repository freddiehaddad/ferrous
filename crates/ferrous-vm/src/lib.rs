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

use log::info;

pub struct VmConfig {
    pub memory_size: usize,
}

pub struct VirtualMachine {
    pub cpu: Cpu,
    pub memory: Box<dyn Memory>,
    pub trap_handler: Box<dyn TrapHandler>,
    // devices: DeviceManager, // For later
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
        })
    }

    pub fn load_program(&mut self, binary: &[u8], entry_point: VirtAddr) -> Result<(), VmError> {
        // For now, assume physical == virtual and load at entry point
        // In reality, we should load segments.
        // For simple bare metal, we might just load at 0x8000_0000

        // This is a simplification. The runtime loader should handle segment loading into memory.
        // The VM just provides memory.
        // But the `load_program` method in ARCHITECTURE.md signature takes `binary` slice.
        // Let's assume the binary is a flat binary for now, or the caller handles parsing.
        // Actually, ARCHITECTURE.md says `load_program(binary: &[u8], entry_point: VirtAddr)`

        let phys_addr = PhysAddr::new(entry_point.val());
        // We need to cast memory to something that can load bytes if it's not exposed in trait
        // But our SimpleMemory has a load method.
        // The trait Memory doesn't have `load`.
        // I should probably just expose `write_byte` loop here.

        for (i, &byte) in binary.iter().enumerate() {
            self.memory.write_byte(phys_addr + (i as u32), byte)?;
        }

        self.cpu.pc = entry_point.val();

        Ok(())
    }

    pub fn run(&mut self) -> Result<ExitReason, VmError> {
        loop {
            match self.step() {
                Ok(StepResult::Continue) => continue,
                Ok(StepResult::Exit(reason)) => return Ok(reason),
                Ok(StepResult::Trap(cause)) => {
                    // Handle trap
                    let resume_addr = self.trap_handler.handle_trap(
                        cause,
                        &mut self.cpu,
                        self.memory.as_mut(),
                    )?;
                    self.cpu.pc = resume_addr.val();
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn step(&mut self) -> Result<StepResult, VmError> {
        // Fetch
        let pc = PhysAddr::new(self.cpu.pc);
        let instruction_word = self.memory.read_word(pc).map_err(VmError::Memory)?;

        // Decode
        let instruction = Instruction::decode(instruction_word)?;

        // Execute
        self.cpu.pc += 4; // Advance PC *before* execution (standard RISC-V, except jumps overwrite it)

        match instruction {
            Instruction::Lui { rd, imm } => {
                self.cpu.write_reg(rd, imm);
            }
            Instruction::Auipc { rd, imm } => {
                let val = pc.val().wrapping_add(imm);
                self.cpu.write_reg(rd, val);
            }
            Instruction::Jal { rd, offset } => {
                let target = pc.val().wrapping_add(offset as u32);
                self.cpu.write_reg(rd, pc.val() + 4); // Link is next instruction (already incremented? No, wait. PC was fetched at `pc`. Next is `pc+4`.)
                                                      // Actually I already incremented self.cpu.pc += 4 above.
                                                      // So rd gets `pc + 4` (which is current self.cpu.pc).
                                                      // And we jump to target.

                // Wait, standard behavior:
                // JAL rd, offset
                // rd = pc + 4
                // pc += offset

                // Since I incremented pc by 4 already, `self.cpu.pc` is now `old_pc + 4`.
                // So `rd` gets `self.cpu.pc`. Correct.
                // But target is `old_pc + offset`.
                // So I need to use `pc.val()` (old pc) + offset.

                self.cpu.write_reg(rd, self.cpu.pc); // Link address
                self.cpu.pc = target; // Jump
            }
            Instruction::Jalr { rd, rs1, offset } => {
                let base = self.cpu.read_reg(rs1);
                let target = base.wrapping_add(offset as u32) & !1; // LSB set to 0

                self.cpu.write_reg(rd, self.cpu.pc); // Link address (old_pc + 4)
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
            Instruction::Addi { rd, rs1, imm } => {
                let val = self.cpu.read_reg(rs1).wrapping_add(imm as u32);
                self.cpu.write_reg(rd, val);
            }
            Instruction::Ecall => {
                // Trap!
                // We need to back up PC?
                // Exceptions usually report the *faulting* PC.
                // ECALL is an exception.
                // So we probably want to report `pc` (the address of the ecall instruction).
                // But we already incremented PC.
                // So we should report `pc.val()`.

                // But wait, if we trap, the handler might decide to resume at `epc + 4`.
                // If we pass `pc.val()` (address of ecall), the handler sees "Oh, ecall at X".
                // If it resumes at X, it executes ecall again -> loop.
                // So it should resume at X+4.

                // The TrapCause logic usually just says "ECALL happened".
                // The handler logic (in kernel) decides the resume address (usually epc + 4).

                // However, my `handle_trap` returns a `VirtAddr` to resume at.
                // So I can just pass the cause.
                // The CPU state `pc` is currently `pc + 4` (next instruction).
                // If I trigger a trap, I usually want `mepc` (Exception PC) to be the address of the instruction that caused it.
                // So I should probably set `self.cpu.pc` back to `pc.val()` before trapping?
                // Or let the trap handler know.

                // Let's set `pc` back to the instruction address for the trap handler context,
                // because standard RISC-V hardware sets `mepc` to the instruction address.
                self.cpu.pc = pc.val();

                let cause = match self.cpu.mode {
                    PrivilegeMode::User => TrapCause::EnvironmentCallFromU,
                    PrivilegeMode::Supervisor => TrapCause::EnvironmentCallFromS,
                    PrivilegeMode::Machine => TrapCause::EnvironmentCallFromS, // Treat as S for now or add M
                };
                return Ok(StepResult::Trap(cause));
            }
            // TODO: Implement others
            _ => {
                // For unimplemented instructions in this iteration
                return Err(VmError::Decode(DecodeError::InvalidOpcode(
                    instruction_word,
                )));
            }
        }

        Ok(StepResult::Continue)
    }
}
