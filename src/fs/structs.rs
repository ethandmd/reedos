use crate::alloc::{boxed::Box, vec::Vec, vec, string::String};
use crate::device::virtio::*;
//use crate::vm::request_phys_page;
use crate::fs::{EXT2_HINT, FsError, Hint};
use core::mem::size_of;
use core::cmp;

const EXT2_MAGIC: u16 = 0xef53;
const EXT2_START_SUPERBLOCK: u64 = 1024; 
const EXT2_END_SUPERBLOCK: u64 = 2048;

/// EXT2 Superblock. Graciously borrowed from @dylanmc.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
// https://wiki.osdev.org/Ext2
pub struct Superblock {
    // taken from https://wiki.osdev.org/Ext2
    /// Total number of inodes in file system
    inodes_count: u32,
    /// Total number of blocks in file system
    blocks_count: u32,
    /// Number of blocks reserved for superuser (see offset 80)
    r_blocks_count: u32,
    /// Total number of unallocated blocks
    free_blocks_count: u32,
    /// Total number of unallocated inodes
    free_inodes_count: u32,
    /// Block number of the block containing the superblock
    first_data_block: u32,
    /// log2 (block size) - 10. (In other words, the number to shift 1,024
    /// to the left by to obtain the block size)
    log_block_size: u32,
    /// log2 (fragment size) - 10. (In other words, the number to shift
    /// 1,024 to the left by to obtain the fragment size)
    log_frag_size: i32,
    /// Number of blocks in each block group
    blocks_per_group: u32,
    /// Number of fragments in each block group
    frags_per_group: u32,
    /// Number of inodes in each block group
    inodes_per_group: u32,
    /// Last mount time (in POSIX time)
    mtime: u32,
    /// Last written time (in POSIX time)
    wtime: u32,
    /// Number of times the volume has been mounted since its last
    /// consistency check (fsck)
    mnt_count: u16,
    /// Number of mounts allowed before a consistency check (fsck) must be
    /// done
    max_mnt_count: i16,
    /// Ext2 signature (0xef53), used to help confirm the presence of Ext2
    /// on a volume
    magic: u16,
    /// File system state (see `FS_CLEAN` and `FS_ERR`)
    state: u16,
    /// What to do when an error is detected (see `ERR_IGNORE`, `ERR_RONLY` and
    /// `ERR_PANIC`)
    errors: u16,
    /// Minor portion of version (combine with Major portion below to
    /// ruct full version field)
    rev_minor: u16,
    /// POSIX time of last consistency check (fsck)
    lastcheck: u32,
    /// Interval (in POSIX time) between forced consistency checks (fsck)
    checkinterval: u32,
    /// Operating system ID from which the filesystem on this volume was
    /// created
    creator_os: u32,
    /// Major portion of version (combine with Minor portion above to
    /// ruct full version field)
    rev_major: u32,
    /// User ID that can use reserved blocks
    block_uid: u16,
    /// Group ID that can use reserved blocks
    block_gid: u16,

    /// First non-reserved inode in file system.
    first_inode: u32,
    /// Size of each inode structure in bytes. - only 128 bytes seem used
    /// but modern EXT filesystems seem to use 256 bytes for each inode
    inode_size: u16,
    /// Block group that this superblock is part of (if backup copy)
    block_group: u16,
    /// Optional features present (features that are not required to read
    /// or write, but usually result in a performance increase)
    features_opt: u32,
    /// Required features present (features that are required to be
    /// supported to read or write)
    features_req: u32,
    /// Features that if not supported, the volume must be mounted
    /// read-only)
    features_ronly: u32,
    /// File system ID (what is output by blkid)
    fs_id: [u8; 16],
    /// Volume name (C-style string: characters terminated by a 0 byte)
    volume_name: [u8; 16],
    /// Path volume was last mounted to (C-style string: characters
    /// terminated by a 0 byte)
    last_mnt_path: [u8; 64],
    /// Compression algorithms used (see Required features above)
    compression: u32,
    /// Number of blocks to preallocate for files
    prealloc_blocks_files: u8,
    /// Number of blocks to preallocate for directories
    prealloc_blocks_dirs: u8,
    #[doc(hidden)]
    _unused: [u8; 2],
    /// Journal ID (same style as the File system ID above)
    journal_id: [u8; 16],
    /// Journal inode
    journal_inode: u32,
    /// Journal device
    journal_dev: u32,
    /// Head of orphan inode list
    journal_orphan_head: u32,
}

impl Superblock {
    /// Example usage:
    /// `let sb: Box<Superblock> = Superblock::read();`
    pub fn read() -> Result<Box<Self>, FsError> {
        let len = (size_of::<Self>() + 512) & !511; //Need to be multiple of 512 for blk dev.
        let mut buf: Vec<u8> = Vec::with_capacity(len);
        let _ = Block::new(buf.as_mut_ptr(), len as u32, EXT2_START_SUPERBLOCK).unwrap().read();
        let raw = buf.as_mut_ptr() as *mut Self;
        let sb = unsafe { *raw };
        
        if sb.magic != EXT2_MAGIC { 
            Err(FsError::BadMagic)
        } else if (1024 << sb.log_block_size) % 1024 != 0 { 
            Err(FsError::BadBlockSize)
        } else if sb.blocks_count / sb.blocks_per_group < 1 { 
            Err(FsError::NoBlocksPerGroup)
        } else {
            Ok(Box::new(sb))
        }
    }

    pub fn build_hint(&self) -> Hint {
        let bsize = 1024 << self.log_block_size;
        let bgd_start = if bsize == 1024 { 2048_u64 } else { bsize as u64 };
        Hint {
            block_size: bsize,
            inode_size: self.inode_size,
            blocks_per_group: self.blocks_per_group,
            inodes_per_group: self.inodes_per_group,
            block_desc_table: Hint::read_bgdt(bgd_start, self.blocks_count.div_ceil(self.blocks_per_group) as usize),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BlockGroupDescriptor {
    /// Block address of block usage bitmap
    block_usage_addr: u32,
    /// Block address of inode usage bitmap
    inode_usage_addr: u32,
    /// Starting block address of inode table
    inode_table_block: u32,
    /// Number of unallocated blocks in group
    free_blocks_count: u16,
    /// Number of unallocated inodes in group
    free_inodes_count: u16,
    /// Number of directories in group
    dirs_count: u16,

    _reserved: [u8; 14],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Inode {
    /// Type and Permissions (see below)
    type_perm: u16, // TypePerm. TODO: Should integrate bitflags! into my life at some point.
    /// User ID
    uid: u16,
    /// Lower 32 bits of size in bytes
    size_low: u32,
    /// Last Access Time (in POSIX time)
    atime: u32,
    /// Creation Time (in POSIX time)
    ctime: u32,
    /// Last Modification time (in POSIX time)
    mtime: u32,
    /// Deletion time (in POSIX time)
    dtime: u32,
    /// Group ID
    gid: u16,
    /// Count of hard links (directory entries) to this inode. When this
    /// reaches 0, the data blocks are marked as unallocated.
    hard_links: u16,
    /// Count of disk sectors (not Ext2 blocks) in use by this inode, not
    /// counting the actual inode structure nor directory entries linking
    /// to the inode.
    sectors_count: u32,
    /// Flags
    flags: u32,
    /// Operating System Specific value #1
    _os_specific_1: [u8; 4],
    /// Direct block pointers
    direct_pointer: [u32; 12],
    /// Singly Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to data)
    indirect_pointer: u32,
    /// Doubly Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to Singly Indirect Blocks)
    doubly_indirect: u32,
    /// Triply Indirect Block Pointer (Points to a block that is a list of
    /// block pointers to Doubly Indirect Blocks)
    triply_indirect: u32,
    /// Generation number (Primarily used for NFS)
    gen_number: u32,
    /// In Ext2 version 0, this field is reserved. In version >= 1,
    /// Extended attribute block (File ACL).
    ext_attribute_block: u32,
    /// In Ext2 version 0, this field is reserved. In version >= 1, Upper
    /// 32 bits of file size (if feature bit set) if it's a file,
    /// Directory ACL if it's a directory
    size_high: u32,
    /// Block address of fragment
    frag_block_addr: u32,
    /// Operating System Specific Value #2
    _os_specific_2: [u8; 12],
    _padding: [u8; 128], // TODO: handle inode sizes != 128 according to superblock
}

impl Inode {
    pub fn read(inum: u32) -> Box<Inode> {
        let hint = unsafe { EXT2_HINT.get().unwrap() };
        // Find which block group to search.
        let block_group = ((inum - 1) / hint.inodes_per_group) as usize;
        // Get bg inode table starting block addr
        let itable = hint.block_desc_table[block_group].inode_table_block;
        // Find index into block group inode table.
        let index = (inum -1) % hint.inodes_per_group;
        // Find which block contains inode.
        let block = (index * hint.inode_size as u32) / hint.block_size;
        // Find byte address
        let offset = ((itable + block) as u64 * hint.block_size as u64) as u64;

        //let buf = request_phys_page(1).unwrap();
        //let mut buf = Vec::with_capacity(size_of::<Inode>() * 2);
        let mut buf = vec![0_u8; size_of::<Inode>() * 2];
        let index_off = (index * hint.inode_size as u32) as u64;
        let offset = (offset + index_off) & !511;
        let _ = Block::new(buf.as_mut_ptr(), buf.capacity() as u32, offset).unwrap().read();
        let raw_off = if index_off % 512 == 0 { 0 } else { hint.inode_size };
        //let raw = unsafe { (buf.start() as *mut Self).add(index as usize) }
        let raw = unsafe { buf.as_ptr().byte_add(raw_off as usize) as *mut Self };
        let inode = unsafe { *raw };
        Box::new(inode)
    }

    pub fn get_type(&self) -> Option<TypePerm> {
        let tp = self.type_perm;
        if tp & (TypePerm::File as u16) == TypePerm::File as u16 {
            Some(TypePerm::File)
        } else if tp & (TypePerm::Directory as u16) == TypePerm::Directory as u16 {
            Some(TypePerm::Directory)
        } else {
            None
        }
    }

    pub fn parse_file(&self, buf: &mut [u8]) -> Result<u32, FsError>{
        let hint = unsafe { EXT2_HINT.get().unwrap() };
        let bsize = hint.block_size;
        let tp = self.type_perm & TypePerm::File as u16;
        if tp != TypePerm::File as u16 {
            return Err(FsError::IncorrectParseType);
        }
        let buf_len = buf.len() as u32;
        let buf_ptr = buf.as_mut_ptr();
        let mut nread: u32 = 0;
        for p in self.direct_pointer {
            if p != 0 {
                if nread >= buf_len { break; }
                let buf_ptr = unsafe { buf_ptr.byte_add(nread as usize) };
                let len = cmp::min(bsize, buf_len - nread);
                let _ = Block::new(buf_ptr, len, (p*bsize) as u64).unwrap().read();
                nread += bsize;
            }
        }
        Ok(nread)
    }

    /// Given a directory inode, parse its entries to discover the contents of the dir
    /// by iterating the non-zero direct pointers.
    /// TODO: Support single indirect pointers.
    /// Double or triple indirect pointer support is not implemented at this time.
    pub fn parse_dir(&self) -> Result<Vec<DirPair>, FsError> {
        let hint = unsafe { EXT2_HINT.get().unwrap() };
        let bsize = hint.block_size as usize;
        let mut buf: Vec<u8> = vec![0; bsize];

        let tp = self.type_perm & TypePerm::Directory as u16;
        if tp != TypePerm::Directory as u16 {
            return Err(FsError::IncorrectParseType);
        }

        let mut ret = Vec::new();
        for p in self.direct_pointer {
            if p != 0 {
                let _ = Block::new(buf.as_mut_ptr(), bsize as u32, (p * bsize as u32) as u64).unwrap().read();
                let ptr = buf.as_mut_ptr();
                let mut idx = 0;
                while idx < bsize {
                    let de = unsafe { ptr.byte_add(idx) as *mut DirectoryEntry };
                    let inode = unsafe { (*de).inode };
                    let nsize = unsafe { (*de).name_length };
                    let dname = unsafe { &mut (*de).name as *const u8};
                    let dsize = unsafe { (*de).entry_size };
                    let nvec = unsafe { core::slice::from_raw_parts(dname, nsize as usize) };
                        let dname = String::from_utf8(nvec.to_vec()).unwrap(); //Self::build_name(nvec);
                    if inode != 0 {
                        ret.push(DirPair::new(dname, inode));
                    }
                    idx += dsize as usize;
                }
            }
        }
        Ok(ret)
    }
}

#[derive(Debug)]
pub struct DirPair {
    pub name: String,
    pub inode: u32,
}

impl DirPair {
    pub fn new(name: String, inode: u32) -> Self {
        Self { name, inode }
    }
}

// Linked List directory entry.
#[repr(C)]
#[derive(Debug)] //, Copy, Clone)]
struct DirectoryEntry {
    /// Inode
    inode: u32,
    /// Total size of this entry (Including all subfields)
    /// (offset to start of next entry)
    entry_size: u16,
    /// Name Length least-significant 8 bits
    name_length: u8,
    /// Type indicator (only if the feature bit for "directory entries have file type byte" is set, else this is the most-significant 8 bits of the Name Length)
    type_indicator: u8,

    name: u8, // Read in byte slice and do str::from_utf8()
}

#[repr(C)]
#[derive(Debug)]
enum TypeIndicator {
    Unknown = 0,
    Regular = 1,
    Directory = 2,
    Character = 3,
    Block = 4,
    Fifo = 5,
    Socket = 6,
    Symlink = 7,
}

impl Default for TypeIndicator {
    fn default() -> TypeIndicator {
        TypeIndicator::Unknown
    }
}

#[repr(u16)]
pub enum TypePerm {
    /// FIFO
    Fifo = 0x1000,
    /// Character device
    CharDevice = 0x2000,
    /// Directory
    Directory = 0x4000,
    /// Block device
    BlockDevice = 0x6000,
    /// Regular file
    File = 0x8000,
    /// Symbolic link
    Symlink = 0xA000,
    /// Unix socket
    Socket = 0xC000,
    /// Other—execute permission
    OExec = 0x001,
    /// Other—write permission
    OWrite = 0x002,
    /// Other—read permission
    ORead = 0x004,
    /// Group—execute permission
    GExec = 0x008,
    /// Group—write permission
    GWrite = 0x010,
    /// Group—read permission
    GRead = 0x020,
    /// User—execute permission
    UExec = 0x040,
    /// User—write permission
    UWrite = 0x080,
    /// User—read permission
    URead = 0x100,
    /// Sticky Bit
    Sticky = 0x200,
    /// Set group ID
    SetGid = 0x400,
    /// Set user ID
    SetUid = 0x800,
}
