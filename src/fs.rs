//! EXT2 Filesystem implementation. Everything is done in byte offsets from start of device
//! until we find this to be cumbersome and use block numbers instead.
//! [ext2 gnu docs](https://www.nongnu.org/ext2-doc/ext2.pdf)

pub mod structs;

use structs::*;
use crate::alloc::{boxed::Box, vec::Vec, vec, string::{String, ToString}};
use crate::device::virtio::Block;
use core::cell::OnceCell;
use core::mem::size_of;

const EXT2_END_SUPERBLOCK: u64 = 2048;
const EXT2_ROOT_INODE: u32 = 2;
pub static mut EXT2_HINT: OnceCell<Hint> = OnceCell::new();

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
    pub fn init() -> Result<(), FsError> {
        let sb: Box<Superblock> = Superblock::read()?;
        let hint = sb.build_hint();
        unsafe {
            match EXT2_HINT.set(hint) {
                Ok(_) => Ok(()),
                Err(_) => Err(FsError::UnableToReadHint),
            }
        }
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
#[derive(Debug)]
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
    path: FilePath,
    inode: Box<Inode>,
    cursor: u32,
}
        
impl FileHandle {
    /// All filepaths must be absolute.
    /// TODO: Make this less unpleasant to look at.
    pub fn open<T: ToString + ?Sized>(path: &T) -> Result<Self, FsError> {
        let path = FilePath::new(path.to_string());
        let working= path.inner.get(1..).unwrap().to_string();
        let mut inode = Inode::read(EXT2_ROOT_INODE);
        let mut dir = inode.parse_dir()?;
        for sub in working.split("/").into_iter() {
            if let Some(inum) = Self::linear_search_dir(sub, &dir) {
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
        Ok(Self { path, inode, cursor: 0 })
    }

    fn linear_search_dir(tgt: &str, slice: &[DirPair]) -> Option<u32> {
        for e in slice {
            if tgt == e.name {
                return Some(e.inode);
            }
        }
        None
    }
}

pub fn play_ext2() {
    let fd = FileHandle::open("/bin/spin.elf").unwrap();
    println!("My first fd: {:?}", fd);
    
    // This doesn't work. Block I/O error.
    //let fd1 = FileHandle::open("/bin").unwrap();
    //println!("My second fd: {:?}", fd1);
}
