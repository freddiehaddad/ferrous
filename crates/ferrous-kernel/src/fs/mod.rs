use crate::error::KernelError;
use ferrous_fs::{DirEntry, Inode, SuperBlock, BLOCK_SIZE, INODE_DIRECT_POINTERS, MAGIC};
use ferrous_vm::Memory;
use log::{debug, error, info};

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
        // We use unsafe cast or manual parsing because we don't have bincode in no_std kernel efficiently yet
        // (unless we add bincode dependency to kernel too, which might be heavy or require alloc)
        // Let's use unsafe cast for now as both are repr(C)

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
        // Inode 0 is at start of inode_table_block
        let inode_size = core::mem::size_of::<Inode>() as u32;
        let inodes_per_block = BLOCK_SIZE as u32 / inode_size;

        let block_offset = inode_id / inodes_per_block;
        let index_in_block = inode_id % inodes_per_block;

        let block_id = self.superblock.inode_table_block + block_offset;

        let mut buffer = [0u8; BLOCK_SIZE];
        block::read_sector(memory, block_id, &mut buffer)
            .map_err(|e| KernelError::InitializationError(format!("Inode Read Error: {}", e)))?;

        let inode = unsafe {
            let ptr = buffer.as_ptr().add((index_in_block * inode_size) as usize) as *const Inode;
            ptr.read_unaligned()
        };

        Ok(inode)
    }
}
