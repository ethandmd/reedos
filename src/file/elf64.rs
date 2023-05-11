//! This module is for the interpretation of 64 bit ELF executable files.

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Endianness {
    Little = 1,
    Big = 2,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum AddrWidth {
    Word = 1,
    DoubleWord = 2,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum ELFType {
    Relocatable = 1,
    Executable = 2,
    Shared = 3,
    Core = 4,
}

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum Architecture {
    Unspecified = 0,
    Sparc = 2,
    X86 = 3,
    Mips = 8,
    PowerPC = 0x14,
    Arm = 0x28,
    SuperH = 0x2A,
    IA64 = 0x32,
    X86_64 = 0x3E,
    Aarch64 = 0xB7,
    RISCV = 0xF3,
}


/// Corresponds to the literal bits of the header. Not all values are
/// meaningful, and `usize` is used when 32 and 64 bit ELF files
/// differ.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ELFHeader {
    pub magic: [u8; 4],
    pub width: AddrWidth,
    pub endian: Endianness,
    pub header_version: u8,                // ELF header version / calling convetion
    pub padding: [u8; 8],                  // first byte is OS/ABI info
    pub ident_size: u8,             // possibly unused
    // end of identifying info
    pub elf_type: ELFType,
    pub instruction_set: Architecture,
    pub version: u32,
    pub entry: usize,
    pub program_header_pos: usize,
    pub section_header_pos: usize, // table index starts at 1
    pub flags: u32,                 // architecture dependent
    pub header_size: u16,
    pub program_entry_size: u16,
    pub num_program_entries: u16,
    pub section_entry_size: u16,
    pub num_section_entries: u16,
    pub section_name_index: u16,
    // index of section string table of names of sections
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProgramSegmentType {
    Ignore = 0,
    Load = 1,
    Dynamic = 2,
    Interpreter = 3,
    Notes = 4,
    Reserved = 5,
    ProgramTable = 6,
    // if there is a segment that points to the table itself
}

pub const PROG_SEG_EXEC: u16 = 1;
pub const PROG_SEG_WRITE: u16 = 2;
pub const PROG_SEG_READ: u16 = 4;

/// A sub header of an ELF file describing the file itself. Also
/// corresponds to the literal bits from the file. Not all values are
/// valid.
///
/// This is a seperate struct from the 32 bit version as their length
/// and arrangment are different enough to warrant it.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ProgramHeaderSegment64 {
    pub seg_type: ProgramSegmentType,
    pub flags: u32,                 // OR of PROG_SEG_*
    pub file_offset: u64,
    pub vmem_addr: u64,
    pub unused: u64,                // for System V ABI anyway, would be phys addr
    pub size_in_file: u64,
    pub size_in_memory: u64,
    pub alignment: u64,              // is a power of two
}

// TODO sections are not implemented currently, as we are interested
// in execution and not linking / relocation

// todo think about if there is a nice streaming solution that avoids
// the intermediate copy

// /// A place you can get an ELF from, might be a file or network or
// /// static memory or whatever. Offsets are in bytes for all functions.
// pub trait ELFSource {
//     fn get_byte(&self, offset: usize) -> u8;
//     fn get_half_word(&self, offset: usize) -> u16;
//     fn get_word(&self, offset: usize) -> u32;
//     fn get_usize(&self, offset: usize) -> usize;

// }

// impl ELFSource for *const u8 {
//     fn get_byte(&self, offset: usize) -> u8 {
//         unsafe { *self.add(offset) }
//     }

//     fn get_half_word(&self, offset: usize) -> u16 {
//         unsafe { *(self.add(offset) as *const u16) }
//     }

//     fn get_word(&self, offset: usize) -> u32 {
//         unsafe { *(self.add(offset) as *const u32) }
//     }

//     fn get_usize(&self, offset: usize) -> usize {
//         unsafe { *(self.add(offset) as *const usize) }
//     }
// }

/// We expect that the data at source continues to be valid for the entire lifetime of ELFProgram.
///
/// TODO how to do wthat with rust lifetime stuff? Restart my attempt for a source trait?
///
/// TODO consider interactions with lifetimes and if it makes more
/// sense to have a in-memory ELF struct rather than moving only one
/// part into memory at a time. Requires more copying, but allows
/// closing the file / (currently) would avoid having to map the entire
/// file for the full duration of process starting
pub struct ELFProgram {
    pub header: ELFHeader,
    pub source: *const u8,
}

macro_rules! ill_formed {
    () => {
        panic!("ELF is ill-formed.")
    }
}
macro_rules! unsupported {
    () => {
        panic!("ELF is unsupported.")
    }
}

// TODO consider rolling together a bunch of these overlapping error
// types as nested enums, if that is a thing, or otherwise not reuse
// things like "Failed alloc" in multiple places
#[non_exhaustive]
#[derive(Debug)]
pub enum ELFError {
    MappedZeroPage,
    MappedKernelText,
    FailedAlloc,
    FailedMap,
    InequalSizes,               // in_file and in_memory don't match
    ExcessiveAlignment,
}

impl ELFProgram {
    /// Create a new object to represent an ELF program and check that
    /// we can actually run it. We need that the data source backing
    /// this object (probably a file), must exist at least until all
    /// the load segments have been copied into the process address
    /// space.
    pub fn new64(src: *const u8) -> Self {
        let out = Self {
            header: unsafe { *(src as *const ELFHeader) },
            source: src,
        };
        if out.header.magic != [0x7f, 'E' as u8, 'L' as u8, 'F' as u8] {
            ill_formed!();
        }
        // filter out illformed or unsupported ELFs
        match out.header.endian as u8 { // little only
            1 => {},
            2 => {unsupported!()}
            _ => {ill_formed!()},
        };
        match out.header.width as u8 { // 64 bit only
            1 => {unsupported!()},
            2 => {},
            _ => {ill_formed!()},
        };
        match out.header.elf_type as u8 { // executable only
            2 => {},
            1 | 3..=4 => {unsupported!()},
            _ => {ill_formed!()},
        };
        match out.header.instruction_set as u8 { // riscv only
            0 | 2 | 3 | 8 |
            0x14 | 0x28 | 0x2A | 0x32 |
            0x3E | 0xB7 => {unsupported!()},
            0xF3 => {},
            _ => {ill_formed!()},
        };
        // todo? Test header_version and version. header flags maybe?
        out
    }

}
