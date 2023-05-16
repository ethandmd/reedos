//! minimal rust kernel built for (qemu virt machine) riscv.
#![no_std]
#![no_main]
#![feature(lazy_cell)]
#![feature(int_roundings)]
#![feature(pointer_byte_offsets)]
#![feature(error_in_core)]
#![feature(sync_unsafe_cell)]
#![feature(panic_info_message)]
#![feature(strict_provenance)]
#![feature(unsized_fn_params)]
#![feature(box_into_inner)]
#![feature(never_type)]
#![allow(dead_code)]
use core::cell::OnceCell;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;
extern crate alloc;

#[macro_use]
pub mod log;
pub mod asm;
pub mod device;
pub mod hw;
pub mod lock;
pub mod trap;
pub mod vm;
pub mod process;
pub mod file;
pub mod fs;


use crate::hw::hartlocal;
use crate::vm::ptable::PageTable;
use crate::device::uart;
use crate::device::plic;
use crate::hw::param;
use crate::hw::riscv::*;
use crate::lock::condition::ConditionVar;

// sync init accross harts
static mut GLOBAL_INIT_FLAG: MaybeUninit<ConditionVar> = MaybeUninit::uninit();
// pass the initial kernel page table to non-zero id harts. This is
// not how it is accessed after inialization
static mut KERNEL_PAGE_TABLE: OnceCell<PageTable> = OnceCell::new();

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let default = format_args!("No message provided");
    let msg = match info.message() {
        Some(msg) => msg,
        None => &default,
    };
    match info.location() {
        None => {
            println!("PANIC! {} at {}", msg, "No location provided");
        }
        Some(loc) => {
            println!("PANIC! {} at {}:{}", msg, loc.file(), loc.line());
        }
    }
    loop {}
}

/// This gets called from entry.S and runs on each hart.
/// Run configuration steps that will allow us to run the
/// kernel in supervisor mode.
#[no_mangle]
pub extern "C" fn _start() {
    // xv6-riscv/kernel/start.c
    let fn_main = main as *const ();

    // Set the *prior* privilege mode to supervisor.
    // Bits 12, 11 are for MPP. They are WPRI.
    // For sstatus we can write SPP reg, bit 8.
    let mut ms = read_mstatus();
    ms &= !MSTATUS_MPP_MASK;
    ms |= MSTATUS_MPP_S;
    ms |= SSTATUS_SUM;          // allow sup access to user pages
    write_mstatus(ms);

    // Set machine exception prog counter to
    // our main function for later mret call.
    write_mepc(fn_main);

    // Disable paging while setting up.
    write_satp(0);

    // Delegate trap handlers to kernel in supervisor mode.
    // Write 1's to all bits of register and read back reg
    // to see which positions hold a 1.
    write_medeleg(0xffff);
    write_mideleg(0xffff);
    //Supervisor interrupt enable.
    let sie = read_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE;
    write_sie(sie);

    // Now give sup mode access to phys mem.
    // Check 3.7.1 of riscv priv isa manual.
    write_pmpaddr0(0x3fffffffffffff_u64); // RTFM
    write_pmpcfg0(0xf); // 1st 8 bits are pmp0cfg

    // Store each hart's hartid in its tp reg for identification.
    let hartid = read_mhartid();
    write_tp(hartid);

    // Get interrupts from clock and set mtev handler fn.
    hw::timerinit();

    // Now return to sup mode and jump to main().
    call_mret();
}

// Primary kernel bootstrap function.
// We ensure that we only initialize kernel subsystems
// one time by only doing so on hart0.
fn main() -> ! {
    // We only bootstrap on hart0.
    let id = read_tp();

    unsafe {
        // this happens on all harts, but they are all writing the same thing
        GLOBAL_INIT_FLAG.write(ConditionVar::new(0));
    }

    if id == 0 {
        uart::init();
        println!("{}", param::BANNER);
        log!(Info, "Bootstrapping on hart0...");
        trap::init();
        log!(Info, "Finished trap init...");
        match vm::global_init() {
            Ok(pt) => {
                unsafe {
                    match KERNEL_PAGE_TABLE.set(pt) {
                        Ok(()) => {},
                        Err(_) => panic!("Kernel Page Table double init!"),
                    }
                    vm::local_init(KERNEL_PAGE_TABLE.get().unwrap());
                }
            },
            Err(_) => {
                panic!("Failed VM initialization!");
            }
        }
        log!(Info, "Initialized the kernel page table...");
        plic::global_init();
        log!(Info, "Finished plic globl init...");
        unsafe {
            log!(Debug, "Testing page allocation and freeing...");
            vm::test_palloc();
            log!(Debug, "Testing galloc allocation and freeing...");
            vm::test_galloc();
        }
        log!(Debug, "Testing phys page extent allocation and freeing...");
        vm::test_phys_page();
        log!(Debug, "Successful phys page extent allocation and freeing...");
        
        log!(Debug, "Initializing VIRTIO blk device...");
        if let Err(e) = device::virtio::virtio_block_init() {
            println!("{:?}", e);
        }
        log!(Debug, "Initializing EXT2 fs...");
        if let Ok(_) = fs::Hint::init() {
            fs::play_ext2();
        }

        process::init_process_structure();
        hartlocal::hartlocal_info_interrupt_stack_init();
        log!(Debug, "Successfuly initialized the process system...");
        plic::local_init();
        log!(Info, "Finished plic local init hart0...");
        log!(Info, "Completed all hart0 initialization and testing...");

        unsafe {
            // release the waiting harts
            GLOBAL_INIT_FLAG.assume_init_mut().update(1);
        }
    } else {
        // Do the init that can be independent and without global deps.
        trap::init();

        unsafe {
            // spin until the global init is done
            GLOBAL_INIT_FLAG.assume_init_ref().spin_wait(1);
            vm::local_init(KERNEL_PAGE_TABLE.get().unwrap());
        }
        hartlocal::hartlocal_info_interrupt_stack_init();
        plic::local_init();
        log!(Info, "Completed all hart{} local initialization", read_tp());

    }
    
    // we want to test multiple processes with multiple harts
    process::test_multiprocess_syscall();
    //loop {}

    panic!("Reached the end of kernel main! Did the root process not start?");
}
