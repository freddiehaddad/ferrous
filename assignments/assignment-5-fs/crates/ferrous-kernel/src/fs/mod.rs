use crate::error::KernelError;
use alloc::collections::VecDeque;
use alloc::format;
use ferrous_fs::{DirEntry, Inode, SuperBlock, BLOCK_SIZE, INODE_DIRECT_POINTERS, MAGIC};
use ferrous_vm::Memory;
use log::{error, info};

pub mod block;
pub mod syscalls;

/// The global file system handle.
///
/// This struct manages the state of the mounted file system, holding the superblock
/// and providing methods to traverse the directory structure and read file data.
///
/// In Ferrous OS, we use a simplified ext2-like file system (FerrousFS).
pub struct FileSystem {
    /// The superblock contains metadata about the file system (size, inode count, etc.).
    pub superblock: SuperBlock,
}

impl FileSystem {
    /// Mounts the file system from the disk.
    ///
    /// This function reads the first block (sector 0) of the disk, which is expected
    /// to contain the SuperBlock. It validates the magic number to ensure the
    /// disk is formatted with FerrousFS.
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

    /// Reads an Inode from the disk by its ID.
    ///
    /// Inodes are stored in the Inode Table, which starts at `superblock.inode_table_block`.
    /// You need to calculate which block the inode is in, read that block, and then
    /// extract the specific Inode struct from the buffer.
    ///
    /// # Assignment 4
    /// Implement this function to locate and read an Inode from the disk.
    pub fn read_inode(&self, memory: &mut dyn Memory, inode_id: u32) -> Result<Inode, KernelError> {
        // TODO: Assignment 4 - Implement read_inode
        // 1. Check if inode_id is valid.
        // 2. Calculate block offset and index within block.
        // 3. Read the block containing the inode.
        // 4. Extract and return the Inode.
        todo!("Assignment 4: read_inode");
    }

    /// Finds an Inode ID by name within the root directory.
    ///
    /// Currently, FerrousFS only supports a flat directory structure (no subdirectories).
    /// This function scans the root directory (Inode 0) for a directory entry matching `name`.
    ///
    /// # Assignment 4
    /// Implement this function to scan directory blocks and match filenames.
    pub fn find_inode(&self, memory: &mut dyn Memory, name: &str) -> Result<u32, KernelError> {
        // TODO: Assignment 4 - Implement find_inode
        // 1. Read Root Inode (ID 0).
        // 2. Iterate through direct pointers of Root Inode.
        // 3. Read directory blocks.
        // 4. Iterate through DirEntries in each block.
        // 5. Match name and return inode_id.
        todo!("Assignment 4: find_inode");
    }

    /// Reads data from a file (Inode).
    ///
    /// This function handles the logic of mapping a logical file offset to actual disk blocks.
    /// It must handle:
    /// 1. Direct pointers (blocks 0-11)
    /// 2. Indirect pointers (block 12) - optional/bonus for Assignment 4?
    ///
    /// # Assignment 4
    /// Implement the logic to read file data, handling block lookups and offsets.
    pub fn read_data(
        &self,
        memory: &mut dyn Memory,
        inode: &Inode,
        offset: u32,
        buffer: &mut [u8],
    ) -> Result<usize, KernelError> {
        // TODO: Assignment 4 - Implement read_data
        // 1. Check EOF.
        // 2. Loop until buffer is full or EOF.
        // 3. Calculate block index and offset in block.
        // 4. Resolve logical block index to physical block ID (Direct/Indirect).
        // 5. Read block (or handle sparse block).
        // 6. Copy data to buffer.
        todo!("Assignment 4: read_data");
    }
}

pub struct Pipe {
    pub buffer: VecDeque<u8>,
    pub read_open: bool,
    pub write_open: bool,
    pub wait_queue: VecDeque<crate::types::ThreadHandle>,
}
