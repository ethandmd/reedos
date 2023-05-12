//! EXT2 Filesystem implementation. Everything is done in byte offsets from start of device
//! until we find this to be cumbersome and use block numbers instead.

pub mod structs;

use structs::*;
use crate::alloc::boxed::Box;

/// EXT2 Block Size can be 1k, 2k, 4k
pub const EXT2_BLOCK_SIZE: u32 = 4096;

const EXT2_MAGIC: u16 = 0xef53;
const EXT2_START_SUPERBLOCK: u64 = 1024;
const EXT2_START_BLOCK_DESC_GROUP: u64 = EXT2_BLOCK_SIZE as u64;

pub enum FsError {
    BadBlockSize,
    BadMagic,
    NoBlocksPerGroup,
}

pub fn init_ext2() -> Result<Box<Superblock>, FsError> {
    //let mut sup_slice = Box::new([0_u8; 1024]);
    //let mut b1 = Block::new(sup_slice.as_mut_ptr(), 1024, 1024).expect("Couldn't allocate fs superblock buffer!");
    //b1.read();
    //let sb: Superblock = unsafe { *sup_slice.as_ptr().cast::<Superblock>() };
    let sb: Box<Superblock> = Superblock::read(1024);
    let mut buf = crate::alloc::vec![0_u8; 4096];
    crate::device::virtio::Block::new(buf.as_mut_ptr(), 4096, 4096).unwrap().read();
    //let mut b2 = Block::new(sup_slice.as_mut_ptr(), 512, EXT2_BLOCK_SIZE as u64).unwrap();
    //b2.read();
    //let bgd: BlockGroupDescriptor = unsafe { *sup_slice.as_ptr().cast::<BlockGroupDescriptor>() };
    //println!("{:?}", bgd);
    
    if sb.magic != EXT2_MAGIC {
        Err(FsError::BadMagic)
    } else if 1024 << sb.log_block_size != EXT2_BLOCK_SIZE {
        Err(FsError::BadBlockSize) 
    } else if sb.blocks_count / sb.blocks_per_group < 1 {
        Err(FsError::NoBlocksPerGroup)
    } else {
        Ok( sb )
    }
}
