#![no_std]

use serde::{Deserialize, Serialize};

pub const BLOCK_SIZE: usize = 512;
pub const MAGIC: u32 = 0xF3AA_C0DE;
pub const INODE_DIRECT_POINTERS: usize = 12;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_block: u32,
    pub data_bitmap_block: u32,
    pub inode_table_block: u32,
    pub data_blocks_start: u32,
    pub total_inodes: u32,
    pub free_inodes: u32,
    pub free_blocks: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum FileType {
    File = 1,
    Directory = 2,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct Inode {
    pub id: u32,
    pub file_type: FileType,
    pub size: u32,
    pub direct_ptrs: [u32; INODE_DIRECT_POINTERS],
    pub indirect_ptr: u32, // Points to a block containing more pointers
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

// Directory Entry (Fixed size for now: 32 bytes)
// Filename: 28 bytes, Inode: 4 bytes
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[repr(C)]
pub struct DirEntry {
    pub inode_id: u32,
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

    pub fn name_as_str(&self) -> &str {
        // Find null terminator or end
        let end = self.name.iter().position(|&c| c == 0).unwrap_or(28);
        core::str::from_utf8(&self.name[0..end]).unwrap_or("<invalid>")
    }
}
