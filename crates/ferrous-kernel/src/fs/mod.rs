use crate::error::KernelError;
use alloc::format;
use ferrous_fs::{DirEntry, Inode, SuperBlock, BLOCK_SIZE, INODE_DIRECT_POINTERS, MAGIC};
use ferrous_vm::Memory;
use log::{error, info};

pub mod block;

pub struct FileSystem {
    pub superblock: SuperBlock,
}

impl FileSystem {
    pub fn mount(memory: &mut dyn Memory) -> Result<Self, KernelError> {
        let mut buffer = [0u8; BLOCK_SIZE];

        // Read Superblock (Sector 0)
        block::read_sector(memory, 0, &mut buffer)
            .map_err(|e| KernelError::InitializationError(format!("FS Mount Error: {}", e)))?;

        // Deserialize Superblock
        let superblock = unsafe {
            let ptr = buffer.as_ptr() as *const SuperBlock;
            ptr.read_unaligned()
        };

        if superblock.magic != MAGIC {
            error!("Invalid Magic: {:#x} != {:#x}", superblock.magic, MAGIC);
            return Err(KernelError::InitializationError(
                "Invalid Filesystem Magic".into(),
            ));
        }

        info!(
            "Mounted FileSystem. Size: {} blocks, Inodes: {}",
            superblock.total_blocks, superblock.total_inodes
        );

        Ok(Self { superblock })
    }

    pub fn read_inode(&self, memory: &mut dyn Memory, inode_id: u32) -> Result<Inode, KernelError> {
        if inode_id >= self.superblock.total_inodes {
            return Err(KernelError::InitializationError(
                "Inode ID out of range".into(),
            ));
        }

        // Calculate block and offset
        let inode_size = core::mem::size_of::<Inode>() as u32;
        let inodes_per_block = BLOCK_SIZE as u32 / inode_size;

        let block_offset = inode_id / inodes_per_block;
        let index_in_block = inode_id % inodes_per_block;

        let block_id = self.superblock.inode_table_block + block_offset;

        let mut buffer = [0u8; BLOCK_SIZE];
        if let Err(e) = block::read_sector(memory, block_id, &mut buffer) {
            return Err(KernelError::InitializationError(format!(
                "Inode Read Error: {}",
                e
            )));
        }

        let inode = unsafe {
            let ptr = buffer.as_ptr().add((index_in_block * inode_size) as usize) as *const Inode;
            ptr.read_unaligned()
        };

        Ok(inode)
    }

    pub fn find_inode(&self, memory: &mut dyn Memory, name: &str) -> Result<u32, KernelError> {
        // Special case for root directory
        if name == "/" {
            return Ok(0);
        }

        // Read root inode (ID 0)
        let root_inode = self.read_inode(memory, 0)?;

        // Scan direct pointers
        for &block_id in root_inode.direct_ptrs.iter() {
            if block_id == 0 {
                continue;
            }

            let mut buffer = [0u8; BLOCK_SIZE];
            if let Err(e) = block::read_sector(memory, block_id, &mut buffer) {
                return Err(KernelError::InitializationError(format!(
                    "Dir Read Error: {}",
                    e
                )));
            }

            // Iterate entries in block
            let entry_size = core::mem::size_of::<DirEntry>();
            let num_entries = BLOCK_SIZE / entry_size;

            for i in 0..num_entries {
                let entry_offset = i * entry_size;
                let entry_ptr = unsafe { buffer.as_ptr().add(entry_offset) as *const DirEntry };
                let entry = unsafe { entry_ptr.read_unaligned() };

                // Skip if name is empty (first char is 0)
                if entry.name[0] == 0 {
                    continue;
                }

                if entry.name_as_str() == name {
                    return Ok(entry.inode_id);
                }
            }
        }

        Err(KernelError::InitializationError("File not found".into()))
    }

    pub fn read_data(
        &self,
        memory: &mut dyn Memory,
        inode: &Inode,
        offset: u32,
        buffer: &mut [u8],
    ) -> Result<usize, KernelError> {
        if offset >= inode.size {
            return Ok(0); // EOF
        }

        let mut bytes_read = 0;
        let mut current_offset = offset;
        let end_offset = (offset + buffer.len() as u32).min(inode.size);

        // While we have bytes to read
        while current_offset < end_offset {
            let block_index = current_offset / BLOCK_SIZE as u32;
            let offset_in_block = (current_offset % BLOCK_SIZE as u32) as usize;
            let bytes_to_read =
                (BLOCK_SIZE - offset_in_block).min((end_offset - current_offset) as usize);

            // Resolve block ID
            let block_id = if (block_index as usize) < INODE_DIRECT_POINTERS {
                inode.direct_ptrs[block_index as usize]
            } else {
                let indirect_index = block_index - INODE_DIRECT_POINTERS as u32;
                let pointers_per_block = (BLOCK_SIZE / 4) as u32;

                if indirect_index < pointers_per_block {
                    let indirect_ptr_block = inode.indirect_ptr;
                    if indirect_ptr_block == 0 {
                        0
                    } else {
                        // Read the indirect block
                        let mut indirect_buf = [0u8; BLOCK_SIZE];
                        if let Err(e) =
                            block::read_sector(memory, indirect_ptr_block, &mut indirect_buf)
                        {
                            return Err(KernelError::InitializationError(format!(
                                "Indirect Block Read Error: {}",
                                e
                            )));
                        }

                        // Read u32 from buffer
                        unsafe {
                            let ptr = indirect_buf.as_ptr().add((indirect_index * 4) as usize)
                                as *const u32;
                            ptr.read_unaligned()
                        }
                    }
                } else {
                    return Err(KernelError::InitializationError(
                        "Double indirect pointers not supported yet".into(),
                    ));
                }
            };

            if block_id == 0 {
                // Sparse block (zeros)
                // Just zero the buffer part
                buffer[bytes_read..(bytes_read + bytes_to_read)].fill(0);
            } else {
                // Read actual block
                let mut block_buf = [0u8; BLOCK_SIZE];
                if let Err(e) = block::read_sector(memory, block_id, &mut block_buf) {
                    return Err(KernelError::InitializationError(format!(
                        "Data Read Error: {}",
                        e
                    )));
                }

                // Copy to user buffer
                buffer[bytes_read..(bytes_read + bytes_to_read)].copy_from_slice(
                    &block_buf[offset_in_block..(offset_in_block + bytes_to_read)],
                );
            }

            bytes_read += bytes_to_read;
            current_offset += bytes_to_read as u32;
        }

        Ok(bytes_read)
    }
}
