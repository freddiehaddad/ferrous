use ferrous_vm::{Memory, PhysAddr, VirtAddr, VirtualMachine, VmError};
use goblin::elf;
use std::error::Error;
use std::fs;
use std::path::Path;

pub struct ProgramLoader;

impl ProgramLoader {
    pub fn load_elf(vm: &mut VirtualMachine, elf_path: &Path) -> Result<VirtAddr, Box<dyn Error>> {
        let buffer = fs::read(elf_path)?;
        let elf = elf::Elf::parse(&buffer)?;

        for ph in elf.program_headers.iter() {
            if ph.p_type == elf::program_header::PT_LOAD {
                let start = ph.p_offset as usize;
                let end = start + ph.p_filesz as usize;
                let data = &buffer[start..end];
                let addr = PhysAddr::new(ph.p_vaddr as u32);

                // Load segment into memory
                // vm.memory is Box<dyn Memory>.
                // Need to copy byte by byte as Memory trait only exposes write_byte
                for (i, &byte) in data.iter().enumerate() {
                    vm.memory.write_byte(addr + (i as u32), byte)?;
                }

                // Zero-fill BSS if memsz > filesz
                if ph.p_memsz > ph.p_filesz {
                    let bss_start = ph.p_vaddr + ph.p_filesz;
                    let bss_len = ph.p_memsz - ph.p_filesz;
                    for i in 0..bss_len {
                        vm.memory
                            .write_byte(PhysAddr::new((bss_start + i) as u32), 0)?;
                    }
                }
            }
        }

        Ok(VirtAddr::new(elf.entry as u32))
    }
}
