//! EXT2 Filesystem implementation. Everything is done in byte offsets from start of device
//! until we find this to be cumbersome and use block numbers instead.
//! [ext2 gnu docs](https://www.nongnu.org/ext2-doc/ext2.pdf)

pub mod structs;

use structs::*;
use crate::alloc::{boxed::Box, vec::Vec, vec, string::{String, ToString}};
use crate::device::virtio::Block;
use core::cell::LazyCell;
use core::mem::size_of;

const EXT2_END_SUPERBLOCK: u64 = 2048;
const EXT2_ROOT_INODE: u32 = 2;
pub const EXT2_HINT: LazyCell<Hint> = LazyCell::new(|| { Hint::init().unwrap() } );

#[derive(Debug)]
pub enum FsError {
    BadBlockSize,
    BadMagic,
    DoesNotExist,
    IncorrectParseType,
    NoBlocksPerGroup,
    UnableToReadHint,
}

pub struct Hint {
    block_size: u32,
    inode_size: u16,
    blocks_per_group: u32,
    inodes_per_group: u32,
    block_desc_table: Vec<BlockGroupDescriptor>,
}

impl Hint {
    pub fn init() -> Result<Hint, FsError> {
        let sb: Box<Superblock> = Superblock::read()?;
        Ok(sb.build_hint())
    }

    // Read block group desc table and just hang on to it in memory.
    pub fn read_bgdt(offset: u64, num: usize) -> Vec<BlockGroupDescriptor> {
        let cap = ((size_of::<BlockGroupDescriptor>() * num) + 512) & !511; //Need to be multiple of 512 for blk dev.
        let buf: Vec<u8> = vec![0; cap]; //Vec::with_capacity(cap);
        let mut buf = core::mem::ManuallyDrop::new(buf);
        let _ = Block::new(buf.as_mut_ptr(), cap as u32, offset).unwrap().read();
        let mut buf = core::mem::ManuallyDrop::new(buf);
        let raw = buf.as_mut_ptr() as *mut BlockGroupDescriptor;
        unsafe { Vec::from_raw_parts(raw, num, cap) }
    }
}

#[repr(transparent)]
struct FilePath {
    inner: String,
}

impl FilePath {
    fn new(inner: String) -> Self {
        Self { inner }
    }
}

#[derive(Debug)]
pub struct FileHandle {
    inode: Box<Inode>,
    cursor: u32,
}
        
impl FileHandle {
    /// All filepaths must be absolute.
    pub fn open<T: ToString + ?Sized>(path: &T) -> Result<Self, FsError> {
        let path = FilePath::new(path.to_string());
        let mut inode = Inode::read(&EXT2_ROOT_INODE);
        let mut dir = inode.parse_dir()?;
        for sub in path.inner.split("/") {
            println!("{}", sub);
            if let Some(inum) = dir.get(sub) {
                inode = Inode::read(inum);
                match inode.get_type().unwrap() {
                    TypePerm::Directory => {
                        dir = inode.parse_dir()?;
                    },
                    TypePerm::File => { break; },
                    _ => {
                        return Err(FsError::IncorrectParseType);
                    },
                }
            } else {
                return Err(FsError::DoesNotExist);
            }
        }
        Ok(Self { inode, cursor: 0 })
    }
}

pub fn play_ext2() {
    //let fd = FileHandle::open("/bin/spin.elf");
    let root_inode = Inode::read(&2);
    let dir = root_inode.parse_dir().unwrap();
    let spin_inode = Inode::read(&12);
    let sbse_inode = Inode::read(&13);
}
