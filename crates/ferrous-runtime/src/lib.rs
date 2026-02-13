pub mod loader;

use ferrous_kernel::Kernel;
use ferrous_vm::{
    devices::{
        block::{SimpleBlockDevice, BLOCK_DEVICE_BASE, BLOCK_DEVICE_SIZE},
        uart::{UartDevice, UART_BASE, UART_SIZE},
    },
    system_bus::SystemBus,
    ExitReason, VirtualMachine, VmConfig, VmError,
};
use std::error::Error;
use std::path::Path;

pub struct Runtime {
    vm: VirtualMachine,
}

impl Runtime {
    pub fn new(memory_size: usize, disk_image: Option<&Path>) -> Result<Self, VmError> {
        let mut bus = SystemBus::new(memory_size);

        // Add UART
        bus.add_device(UART_BASE, UART_SIZE, Box::new(UartDevice::new()));

        // Add Block Device if provided
        if let Some(disk_path) = disk_image {
            let block_dev = SimpleBlockDevice::new(disk_path.to_str().unwrap()).map_err(|e| {
                VmError::Device(ferrous_vm::DeviceError::Io(format!(
                    "Failed to open disk image: {}",
                    e
                )))
            })?;
            bus.add_device(BLOCK_DEVICE_BASE, BLOCK_DEVICE_SIZE, Box::new(block_dev));
        }

        // Create Memory (Boxed)
        let mut memory = Box::new(bus);

        // Kernel::new() returns KernelError, map it?
        let kernel = Kernel::new().map_err(|e| {
            VmError::Device(ferrous_vm::DeviceError::Io(format!(
                "Kernel init failed: {}",
                e
            )))
        })?;

        // Initialize Kernel Page Tables
        // We pass `&mut *memory` to `kernel.init_memory`
        let satp = kernel.init_memory(memory.as_mut()).map_err(|e| {
            VmError::Device(ferrous_vm::DeviceError::Io(format!(
                "Kernel memory init failed: {}",
                e
            )))
        })?;

        let config = VmConfig {
            memory_size,
            timer_interval: Some(100), // Trigger interrupt every 100 instructions
        };

        let mut vm = VirtualMachine::new(config, memory, Box::new(kernel))?;
        vm.cpu.satp = satp;

        Ok(Self { vm })
    }

    pub fn load_program(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let entry_point = loader::ProgramLoader::load_elf(&mut self.vm, path)?;
        self.vm.cpu.pc = entry_point.val();

        // Initialize Stack Pointer to top of memory
        // Assuming base address 0x8000_0000 (should probably be in config)
        let stack_top = 0x8000_0000 + self.vm.config.memory_size as u32;
        self.vm.cpu.write_reg(ferrous_vm::Register::SP, stack_top);

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
