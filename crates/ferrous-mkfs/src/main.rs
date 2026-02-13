use clap::Parser;
use ferrous_fs::{DirEntry, FileType, Inode, SuperBlock, BLOCK_SIZE, INODE_DIRECT_POINTERS, MAGIC};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to the disk image
    #[arg(short, long)]
    disk: PathBuf,

    /// Number of inodes
    #[arg(short, long, default_value_t = 128)]
    inodes: u32,

    /// Force overwrite
    #[arg(short, long)]
    force: bool,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // Open disk image
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true) // Create if not exists (but needs size set if new)
        .open(&cli.disk)?;

    // Get disk size
    let file_len = file.metadata()?.len();
    if file_len < (BLOCK_SIZE * 10) as u64 {
        eprintln!("Disk image too small!");
        std::process::exit(1);
    }

    let total_blocks = (file_len / BLOCK_SIZE as u64) as u32;
    println!(
        "Formatting {} ({} bytes, {} blocks)...",
        cli.disk.display(),
        file_len,
        total_blocks
    );

    // Calculate Layout
    let inode_bitmap_blocks = 1; // Simplify: 1 block for bitmap (covers 4096 inodes)
    let data_bitmap_blocks = 1; // Simplify: 1 block for data bitmap
    let inode_table_size = std::mem::size_of::<Inode>() as u32;
    let inodes_per_block = BLOCK_SIZE as u32 / inode_table_size;
    let inode_table_blocks = (cli.inodes + inodes_per_block - 1) / inodes_per_block;

    // Layout:
    // 0: Superblock
    // 1: Inode Bitmap
    // 2: Data Bitmap
    // 3..3+N: Inode Table
    // Rest: Data

    let sb_block = 0;
    let inode_bitmap_block = 1;
    let data_bitmap_block = 2;
    let inode_table_start = 3;
    let data_blocks_start = inode_table_start + inode_table_blocks;
    let free_blocks = total_blocks - data_blocks_start;

    let sb = SuperBlock {
        magic: MAGIC,
        total_blocks,
        inode_bitmap_block,
        data_bitmap_block,
        inode_table_block: inode_table_start,
        data_blocks_start,
        total_inodes: cli.inodes,
        free_inodes: cli.inodes - 1, // Root is taken
        free_blocks,
    };

    println!("Superblock: {:#?}", sb);

    let config = bincode::config::standard().with_fixed_int_encoding();

    // 1. Write Superblock
    let mut sb_bytes = [0u8; BLOCK_SIZE];
    bincode::serde::encode_into_slice(&sb, &mut sb_bytes, config).unwrap();
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&sb_bytes)?;

    // 2. Init Inode Bitmap (Bit 0 set for Root Inode)
    let mut inode_bitmap = vec![0u8; BLOCK_SIZE];
    inode_bitmap[0] = 1; // Inode 0 is used
    file.seek(SeekFrom::Start(
        (inode_bitmap_block * BLOCK_SIZE as u32) as u64,
    ))?;
    file.write_all(&inode_bitmap)?;

    // 3. Init Data Bitmap (All free)
    let data_bitmap = vec![0u8; BLOCK_SIZE];
    file.seek(SeekFrom::Start(
        (data_bitmap_block * BLOCK_SIZE as u32) as u64,
    ))?;
    file.write_all(&data_bitmap)?;

    // 4. Init Inode Table
    // Create Root Inode (Inode 0) - Directory
    let root_inode = Inode::new(0, FileType::Directory);

    // We need to write root inode to the first slot of inode table
    file.seek(SeekFrom::Start(
        (inode_table_start * BLOCK_SIZE as u32) as u64,
    ))?;
    let mut root_bytes = [0u8; BLOCK_SIZE];
    bincode::serde::encode_into_slice(&root_inode, &mut root_bytes, config).unwrap();
    file.write_all(&root_bytes)?;

    // Zero out rest of inode table
    // (Actually, just ensure the file is zeroed or we write valid inodes later.
    // Since we formatted a new image or zeroed it, it might be fine, but let's be safe for the first block of inodes)

    // 5. Zero out data area (optional, slow for large disks)
    // println!("Zeroing data area...");

    println!("Format complete.");

    Ok(())
}
