pub mod loader;

use ferrous_kernel::Kernel;
use ferrous_vm::{ExitReason, SimpleMemory, VirtualMachine, VmConfig, VmError};
use std::error::Error;
use std::path::Path;

pub struct Runtime {
    vm: VirtualMachine,
}

impl Runtime {
    pub fn new(memory_size: usize) -> Result<Self, VmError> {
        let memory = Box::new(SimpleMemory::new(memory_size));
        // Kernel::new() returns KernelError, map it?
        let kernel = Kernel::new().map_err(|e| {
            VmError::Device(ferrous_vm::DeviceError::Io(format!(
                "Kernel init failed: {}",
                e
            )))
        })?;

        let config = VmConfig { memory_size };

        let vm = VirtualMachine::new(config, memory, Box::new(kernel))?;

        Ok(Self { vm })
    }

    pub fn load_program(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let entry_point = loader::ProgramLoader::load_elf(&mut self.vm, path)?;
        self.vm.cpu.pc = entry_point.val();
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        match self.vm.run() {
            Ok(ExitReason::Halt) => Ok(()),
            Ok(ExitReason::Breakpoint) => {
                println!("Breakpoint hit!");
                Ok(())
            }
            Ok(ExitReason::Error(e)) => Err(Box::new(e)),
            Err(e) => Err(Box::new(e)),
        }
    }
}
