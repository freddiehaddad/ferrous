pub mod loader;

use ferrous_kernel::Kernel;
use ferrous_vm::{
    devices::{
        block::{SimpleBlockDevice, BLOCK_DEVICE_BASE, BLOCK_DEVICE_SIZE},
        net::{SimpleNetDevice, NET_DEVICE_BASE},
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

        // Add Network Device
        // We bind to a random port and connect to the test server at 5555
        let net_dev = SimpleNetDevice::new("127.0.0.1:0", "127.0.0.1:5555").map_err(|e| {
            VmError::Device(ferrous_vm::DeviceError::Io(format!(
                "Failed to init network: {}",
                e
            )))
        })?;
        // 4KB space for registers + buffer
        bus.add_device(NET_DEVICE_BASE, 0x1000, Box::new(net_dev));

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
        let mut kernel = Kernel::new().map_err(|e| {
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
        let elf_data = std::fs::read(path)?;

        // Workaround borrow checker to access Kernel and Memory simultaneously
        let vm_ptr = &mut self.vm as *mut VirtualMachine;
        let kernel = unsafe {
            (*vm_ptr)
                .trap_handler
                .as_any()
                .downcast_mut::<Kernel>()
                .ok_or("Failed to downcast Kernel")?
        };
        let memory = unsafe { (*vm_ptr).memory.as_mut() };

        // Bootstrap the initial process (e.g. Shell)
        // We pass the filename as the first arg (argv[0])
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        let args = vec![filename];

        let (entry, satp, sp, a0, a1) = kernel.bootstrap_process(memory, &elf_data, &args)?;

        self.vm.cpu.pc = entry.val();
        self.vm.cpu.satp = satp;
        self.vm.cpu.mode = ferrous_vm::PrivilegeMode::User;

        // Initialize Stack Pointer (SP - x2)
        self.vm
            .cpu
            .write_reg(ferrous_vm::Register::new(2).unwrap(), sp);
        // Initialize Argument Count (A0 - x10)
        self.vm
            .cpu
            .write_reg(ferrous_vm::Register::new(10).unwrap(), a0);
        // Initialize Argument Vector (A1 - x11)
        self.vm
            .cpu
            .write_reg(ferrous_vm::Register::new(11).unwrap(), a1);

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
