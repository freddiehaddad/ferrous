#![no_std]

use serde::{Deserialize, Serialize};

/// The size of a disk block in bytes (512 bytes).
pub const BLOCK_SIZE: usize = 512;
/// Magic number to identify FerrousFS (0xF3AAC0DE).
pub const MAGIC: u32 = 0xF3AA_C0DE;
/// Number of direct block pointers in an Inode.
pub const INODE_DIRECT_POINTERS: usize = 12;

/// The SuperBlock contains global metadata about the file system.
/// It is always located at the first block (Sector 0) of the disk.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct SuperBlock {
    /// Magic number to verify filesystem type.
    pub magic: u32,
    /// Total number of blocks in the filesystem.
    pub total_blocks: u32,
    /// Block ID where the inode bitmap starts.
    pub inode_bitmap_block: u32,
    /// Block ID where the data block bitmap starts.
    pub data_bitmap_block: u32,
    /// Block ID where the inode table starts.
    pub inode_table_block: u32,
    /// Block ID where the data blocks start.
    pub data_blocks_start: u32,
    /// Total number of inodes in the filesystem.
    pub total_inodes: u32,
    /// Number of free inodes available.
    pub free_inodes: u32,
    /// Number of free data blocks available.
    pub free_blocks: u32,
}

/// Represents the type of a file (Regular File or Directory).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum FileType {
    File = 1,
    Directory = 2,
}

/// An Inode (Index Node) represents a file or directory on the disk.
/// It stores metadata (size, type) and pointers to the data blocks
/// holding the file's content.
///
/// # Assignment 5
/// You will work with this structure to implement file reading.
/// - `direct_ptrs`: Points directly to data blocks.
/// - `indirect_ptr`: Points to a block that contains a list of block pointers.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct Inode {
    /// Unique ID of the inode.
    pub id: u32,
    /// Type of the file (File or Directory).
    pub file_type: FileType,
    /// Size of the file in bytes.
    pub size: u32,
    /// Array of direct block pointers.
    /// If a pointer is 0, it means that part of the file is sparse (all zeros).
    pub direct_ptrs: [u32; INODE_DIRECT_POINTERS],
    /// Pointer to a single indirect block.
    /// This block contains `BLOCK_SIZE / 4` additional block pointers.
    pub indirect_ptr: u32,
}

impl Inode {
    pub fn new(id: u32, file_type: FileType) -> Self {
        Self {
            id,
            file_type,
            size: 0,
            direct_ptrs: [0; INODE_DIRECT_POINTERS],
            indirect_ptr: 0,
        }
    }
}

/// A Directory Entry maps a filename to an Inode ID.
/// Directories in FerrousFS are just files containing a list of these entries.
///
/// Fixed size: 32 bytes (28 bytes for name, 4 bytes for Inode ID).
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct DirEntry {
    /// The Inode ID of the file.
    pub inode_id: u32,
    /// The filename (up to 28 bytes, null-padded).
    pub name: [u8; 28],
}

impl DirEntry {
    pub fn new(inode_id: u32, name_str: &str) -> Self {
        let mut name = [0u8; 28];
        let bytes = name_str.as_bytes();
        let len = bytes.len().min(28);
        name[0..len].copy_from_slice(&bytes[0..len]);
        Self { inode_id, name }
    }

    /// Helper to get the name as a string slice.
    pub fn name_as_str(&self) -> &str {
        // Find null terminator or end
        let end = self.name.iter().position(|&c| c == 0).unwrap_or(28);
        core::str::from_utf8(&self.name[0..end]).unwrap_or("<invalid>")
    }
}
