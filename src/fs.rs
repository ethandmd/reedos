//! EXT2 Filesystem implementation. Everything is done in byte offsets from start of device
//! until we find this to be cumbersome and use block numbers instead.
//! [ext2 gnu docs](https://www.nongnu.org/ext2-doc/ext2.pdf)

pub mod structs;

use structs::*;
use crate::alloc::boxed::Box;
use core::cell::OnceCell;

pub static mut EXT2_HINT: OnceCell<Hint> = OnceCell::new();

const EXT2_MAGIC: u16 = 0xef53;
const EXT2_START_SUPERBLOCK: u64 = 1024;

// How To Read An Inode

// Read the Superblock to find the size of each block, the number of blocks per group, number Inodes per group,
// and the starting block of the first group (Block Group Descriptor Table).
// Determine which block group the inode belongs to.
// Read the Block Group Descriptor corresponding to the Block Group which contains the inode to be looked up.
// From the Block Group Descriptor, extract the location of the block group's inode table.
// Determine the index of the inode in the inode table.
// Index the inode table (taking into account non-standard inode size).

// Directory entry information and file contents are located within the data blocks that the Inode points to.
// How To Read the Root Directory

// The root directory's inode is defined to always be 2. Read/parse the contents of inode 2.

#[derive(Debug)]
pub enum FsError {
    BadBlockSize,
    BadMagic,
    NoBlocksPerGroup,
    UnableToReadHint,
}

pub fn init_ext2() -> Result<(), FsError> {
    let sb: Box<Superblock> = Superblock::read();
    //println!("{:?}", sb);
    if sb.magic != EXT2_MAGIC {
        Err(FsError::BadMagic)
    } else if (1024 << sb.log_block_size) % 1024 != 0 {
        Err(FsError::BadBlockSize)
    } else if sb.blocks_count / sb.blocks_per_group < 1 {
        Err(FsError::NoBlocksPerGroup)
    } else {
        match unsafe { EXT2_HINT.set(Hint::from_super(&sb)) } {
            Ok(_) => Ok(()),
            Err(_) => Err(FsError::UnableToReadHint),
        }
    }
}

pub fn play_ext2() {
    let root_inode = Inode::read(unsafe{ EXT2_HINT.get().unwrap() }, 2);
    let dir = root_inode.parse_dir(unsafe{ EXT2_HINT.get().unwrap() }).unwrap();
    for i in 0..5 {
        println!("Dir: {:?}", dir[i]);
    }
    let spin_inode = Inode::read(unsafe { EXT2_HINT.get().unwrap() }, 12);
    let sbse_inode = Inode::read(unsafe { EXT2_HINT.get().unwrap() }, 13);
}
