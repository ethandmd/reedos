//! This module is for the interpretation of 64 bit ELF executable files.
use core::assert;
use core::mem::size_of;
use crate::vm::process::Process;
use crate::vm::{ptable, request_phys_page};

#[repr(u8)]
#[derive(Copy, Clone)]
enum Endianness {
    Little = 1,
    Big = 2,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum AddrWidth {
    Word = 1,
    DoubleWord = 2,
}

#[repr(u16)]
#[derive(Copy, Clone)]
enum ELFType {
    Relocatable = 1,
    Executable = 2,
    Shared = 3,
    Core = 4,
}

#[repr(u16)]
#[derive(Copy, Clone)]
enum Architecture {
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
struct ELFHeader {
    magic: [u8; 4],
    width: AddrWidth,
    endian: Endianness,
    header_version: u8,                // ELF header version / calling convetion
    padding: [u8; 8],
    ident_size: u8,             // possibly unused
    // end of identifying info
    elf_type: ELFType,
    instruction_set: Architecture,
    version: u16,
    entry: usize,
    program_header_pos: usize,
    section_header_pos: usize,
    flags: u32,                 // architecture dependent
    header_size: u16,
    program_entry_size: u16,
    num_program_entries: u16,
    section_entry_size: u16,
    num_section_entries: u16,
    section_name_index: u16,
    // index of section string table of names of sections
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
enum ProgramSegmentType {
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
const PROG_SEG_WRITE: u16 = 2;
const PROG_SEG_READ: u16 = 4;

/// A sub header of an ELF file describing the file itself. Also
/// corresponds to the literal bits from the file. Not all values are
/// valid.
///
/// This is a seperate struct from the 32 bit version as their length
/// and arrangment are different enough to warrant it.
#[repr(C)]
#[derive(Copy, Clone)]
struct ProgramHeaderSegment64 {
    seg_type: ProgramSegmentType,
    flags: u16,                 // OR of PROG_SEG_*
    file_offset: u64,
    vmem_addr: u64,
    unused: u64,                // for System V ABI anyway, would be phys addr
    size_in_file: u64,
    size_in_memory: u64,
    alignment: u64,              // is a power of two
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

pub struct ELFProgram {
    header: ELFHeader,
    source: *const u8,
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

#[non_exhaustive]
pub enum ELFError {
    MappedZeroPage,
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
        if out.header.magic != [0x74, 'E' as u8, 'L' as u8, 'F' as u8] {
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

    pub fn populate_pagetable64(&self, proc: Process) -> Result<(), ELFError>{
        assert!(self.header.program_entry_size as usize == size_of::<ProgramHeaderSegment64>(),
        "Varying ELF entry size expectations.");

        let num = self.header.num_program_entries;
        let mut ptr = self.header.program_header_pos as *const ProgramHeaderSegment64;
        for i in 0..num {
            let segment = unsafe { *ptr.add(i as usize) };
            if segment.seg_type != ProgramSegmentType::Load {
                continue;
            }
            let n_pages = (segment.size_in_memory + (0x1000 - 1)) >> 12;
            let pages = match request_phys_page(n_pages) {
                Ok(p) => {p},
                Err(e) => {
                    panic!("Could not allocate VM for process, {:?}", e);
                }
            };
            let flags = ptable::user_process_flags(
                segment.flags & PROG_SEG_READ != 0,
                segment.flags & PROG_SEG_WRITE != 0,
                segment.flags & PROG_SEG_EXEC != 0
            );

            match ptable::page_map(
                proc.pgtbl,
                ptable::VirtAddress::from(segment.vmem_addr as *mut usize),
                ptable::PhysAddress::from(pages.start() as *mut usize),
                segment.size_in_memory as usize,
                flags) {
                Ok(_) => {},
                Err(e) => {panic!("Error during process mapping! {:?}", e)}
            }
        }

        Ok(())
    }
}
