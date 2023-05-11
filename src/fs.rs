pub mod structs;

/// EXT2 Block Size can be 1k, 2k, 4k, 8k
pub const EXT2_BLOCK_SIZE: u64 = 1024;

const EXT2_MAGIC: u16 = 0xef53;
const EXT2_START_OF_SUPERBLOCK: usize = 1024;
const EXT2_END_OF_SUPERBLOCK: usize = 2048;
