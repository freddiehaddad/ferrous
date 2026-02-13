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

    /// Files to add (optional)
    #[arg(trailing_var_arg = true)]
    files: Vec<PathBuf>,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // Open disk image
    #[allow(clippy::suspicious_open_options)]
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
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
    let _inode_bitmap_blocks = 1; // Simplify: 1 block for bitmap (covers 4096 inodes)
    let _data_bitmap_blocks = 1; // Simplify: 1 block for data bitmap
    let inode_table_size = std::mem::size_of::<Inode>() as u32;
    let inodes_per_block = BLOCK_SIZE as u32 / inode_table_size;
    let inode_table_blocks = cli.inodes.div_ceil(inodes_per_block);

    // Layout:
    // 0: Superblock
    // 1: Inode Bitmap
    // 2: Data Bitmap
    // 3..3+N: Inode Table
    // Rest: Data

    let _sb_block = 0;
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

    unsafe fn as_bytes<T: Sized>(p: &T) -> &[u8] {
        std::slice::from_raw_parts((p as *const T) as *const u8, std::mem::size_of::<T>())
    }

    // 1. Write Superblock
    let mut sb_bytes = [0u8; BLOCK_SIZE];
    // Use raw memory layout (repr(C)) to match kernel reading
    let raw_sb = unsafe { as_bytes(&sb) };
    sb_bytes[0..raw_sb.len()].copy_from_slice(raw_sb);

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&sb_bytes)?;

    // 2. Init Inode Bitmap (Bit 0 set for Root Inode)
    let mut inode_bitmap = vec![0u8; BLOCK_SIZE];
    inode_bitmap[0] = 1; // Inode 0 is used

    // 3. Init Data Bitmap
    let mut data_bitmap = vec![0u8; BLOCK_SIZE];

    // 4. Init Inode Table Buffer
    let mut inodes = vec![Inode::new(0, FileType::File); cli.inodes as usize];

    // Root Inode (Inode 0)
    let mut root_inode = Inode::new(0, FileType::Directory);

    // Track data allocation
    let mut next_data_block = 0;

    // Allocate Root Directory Block (Block 0)
    let root_block_index = next_data_block;
    next_data_block += 1;

    let mut root_entries = Vec::new();

    // Default README
    let readme_content = b"# Ferrous OS\n\nA simple educational OS in Rust.";
    let readme_inode_idx = 1;

    // Write Readme Content
    let readme_blocks = readme_content.len().div_ceil(BLOCK_SIZE);
    let readme_start = next_data_block;
    next_data_block += readme_blocks as u32;

    for i in 0..readme_blocks {
        let block_abs = data_blocks_start + readme_start + i as u32;
        file.seek(SeekFrom::Start((block_abs * BLOCK_SIZE as u32) as u64))?;
        let start = i * BLOCK_SIZE;
        let end = (start + BLOCK_SIZE).min(readme_content.len());
        let mut block_data = [0u8; BLOCK_SIZE];
        block_data[0..(end - start)].copy_from_slice(&readme_content[start..end]);
        file.write_all(&block_data)?;
    }

    let mut readme_inode = Inode::new(readme_inode_idx, FileType::File);
    readme_inode.size = readme_content.len() as u32;
    for i in 0..readme_blocks {
        readme_inode.direct_ptrs[i] = data_blocks_start + readme_start + i as u32;
    }
    inodes[readme_inode_idx as usize] = readme_inode;
    root_entries.push(DirEntry::new(readme_inode_idx, "README.md"));
    inode_bitmap[0] |= 1 << readme_inode_idx; // Mark used

    // Process additional files
    let mut current_inode = 2;

    for path in &cli.files {
        if path.exists() {
            println!("Adding file: {}", path.display());
            let content = std::fs::read(path)?;
            let blocks_needed = content.len().div_ceil(BLOCK_SIZE);

            // if blocks_needed > 12 {
            //    eprintln!("Warning: Skipping {} (too large, > 12 blocks)", path.display());
            //    continue;
            // }

            let start_block = next_data_block;
            next_data_block += blocks_needed as u32;

            // Write content
            for i in 0..blocks_needed {
                let block_abs = data_blocks_start + start_block + i as u32;
                file.seek(SeekFrom::Start((block_abs * BLOCK_SIZE as u32) as u64))?;
                let start = i * BLOCK_SIZE;
                let end = (start + BLOCK_SIZE).min(content.len());
                let mut block_data = [0u8; BLOCK_SIZE];
                block_data[0..(end - start)].copy_from_slice(&content[start..end]);
                file.write_all(&block_data)?;
            }

            let mut inode = Inode::new(current_inode, FileType::File);
            inode.size = content.len() as u32;

            let mut indirect_needed = false;
            let mut indirect_block_ptr = 0;

            if blocks_needed > INODE_DIRECT_POINTERS {
                indirect_needed = true;
                indirect_block_ptr = data_blocks_start + next_data_block;
                next_data_block += 1;
            }

            for i in 0..blocks_needed {
                let block_abs = data_blocks_start + start_block + i as u32;
                if i < INODE_DIRECT_POINTERS {
                    inode.direct_ptrs[i] = block_abs;
                }
            }

            if indirect_needed {
                inode.indirect_ptr = indirect_block_ptr;
                let mut indirect_block = [0u8; BLOCK_SIZE];
                let offset_in_file_blocks = INODE_DIRECT_POINTERS;
                let remaining_blocks = blocks_needed - offset_in_file_blocks;

                for j in 0..remaining_blocks {
                    let file_block_idx = offset_in_file_blocks + j;
                    let block_abs = data_blocks_start + start_block + file_block_idx as u32;
                    let ptr_offset = j * 4;
                    if ptr_offset + 4 > BLOCK_SIZE {
                        eprintln!("File too large even for indirect block: {}", path.display());
                        break;
                    }
                    indirect_block[ptr_offset..ptr_offset + 4]
                        .copy_from_slice(&block_abs.to_le_bytes());
                }

                // Write indirect block
                file.seek(SeekFrom::Start(
                    (indirect_block_ptr * BLOCK_SIZE as u32) as u64,
                ))?;
                file.write_all(&indirect_block)?;
            }

            inodes[current_inode as usize] = inode;

            let filename = path.file_name().unwrap().to_str().unwrap();
            root_entries.push(DirEntry::new(current_inode, filename));

            // Mark inode used
            let byte = current_inode / 8;
            let bit = current_inode % 8;
            if byte < BLOCK_SIZE as u32 {
                inode_bitmap[byte as usize] |= 1 << bit;
            }

            current_inode += 1;
        } else {
            eprintln!("Warning: File not found: {}", path.display());
        }
    }

    // Write Root Directory Data
    let root_block_abs = data_blocks_start + root_block_index;
    file.seek(SeekFrom::Start((root_block_abs * BLOCK_SIZE as u32) as u64))?;

    let mut root_data = [0u8; BLOCK_SIZE];
    let mut offset = 0;
    for entry in root_entries {
        let raw = unsafe { as_bytes(&entry) };
        if offset + raw.len() > BLOCK_SIZE {
            break; // Directory full (simplification)
        }
        root_data[offset..offset + raw.len()].copy_from_slice(raw);
        offset += raw.len();
    }
    file.write_all(&root_data)?;

    // Update Root Inode
    root_inode.direct_ptrs[0] = root_block_abs;
    root_inode.size = offset as u32; // Size is used bytes
    inodes[0] = root_inode;
    inode_bitmap[0] |= 1; // Root used

    // Update Data Bitmap
    for i in 0..next_data_block {
        let byte = i / 8;
        let bit = i % 8;
        data_bitmap[byte as usize] |= 1 << bit;
    }

    // Write Bitmaps
    file.seek(SeekFrom::Start(
        (inode_bitmap_block * BLOCK_SIZE as u32) as u64,
    ))?;
    file.write_all(&inode_bitmap)?;

    file.seek(SeekFrom::Start(
        (data_bitmap_block * BLOCK_SIZE as u32) as u64,
    ))?;
    file.write_all(&data_bitmap)?;

    // Write Inode Table
    file.seek(SeekFrom::Start(
        (inode_table_start * BLOCK_SIZE as u32) as u64,
    ))?;

    for chunk in inodes.chunks(inodes_per_block as usize) {
        let mut block = [0u8; BLOCK_SIZE];
        let mut offset = 0;
        for inode in chunk {
            let raw = unsafe { as_bytes(inode) };
            block[offset..offset + raw.len()].copy_from_slice(raw);
            offset += raw.len();
        }
        file.write_all(&block)?;
    }

    println!("Format complete. {} files added.", current_inode - 1); // -1 for root

    Ok(())
}
