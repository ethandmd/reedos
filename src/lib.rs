//! minimal rust kernel built for (qemu virt machine) riscv.
#![no_std]
#![no_main]
#![feature(pointer_byte_offsets)]

use core::panic::PanicInfo;

pub mod entry;
#[macro_use]
pub mod log;
pub mod hw;
pub mod lock;
pub mod trap;
pub mod device;

use crate::hw::riscv::*;
use crate::hw::param;
use crate::device::uart;

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// This gets called from src/entry.rs and runs on each hart.
/// Run configuration steps that will allow us to run the 
/// kernel in supervisor mode.
///
/// This is referenced from the xv6-riscv kernel.
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
// one time by only doing so on hart0, and sending
// any other hart to essentially wait for interrupt (wfi).
fn main() -> ! {
    // We only bootstrap on hart0.
    let id = read_tp();
    if id == 0 {
        uart::Uart::init();
        println!("{}", param::BANNER);
        log!(Info, "Bootstrapping on hart0...");
        write_stvec(trap::__strapvec as usize);
    } else {
        write_stvec(trap::__strapvec as usize);
    }

    loop {}
}
