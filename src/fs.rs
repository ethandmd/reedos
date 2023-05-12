pub mod structs;

use structs::*;
use crate::device::virtio::*;
use crate::alloc::boxed::Box;

/// EXT2 Block Size can be 1k, 2k, 4k, 8k
pub const EXT2_BLOCK_SIZE: u64 = 1024;

const EXT2_MAGIC: u16 = 0xef53;
const EXT2_START_OF_SUPERBLOCK: usize = 1024;
const EXT2_END_OF_SUPERBLOCK: usize = 2048;

pub fn init_ext2() -> Result<Superblock, ()> {
    let mut sup_slice = Box::new([0_u8; 1024]);
    let mut b1 = Block::new(sup_slice.as_mut_ptr(), 1024, 1024)?;
    b1.read();
    let sb: *const Superblock = sup_slice.as_ptr().cast::<Superblock>();
    Ok( unsafe { *sb } )
}
