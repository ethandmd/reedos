//! minimal rust kernel built for (qemu virt machine) riscv.
#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod entry;
#[macro_use]
pub mod log;
pub mod param;
pub mod riscv;
pub mod spinlock;
pub mod timervec;
pub mod uart;
use log::*;
use riscv::*;

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Sets up the core local interrupt controller on each hart.
// We set up CLINT per hart before we start bootstrapping so
// we can handle interrupts in supervisor mode (as opposed to
// machine mode).
fn timerinit() {
    let clint = param::CLINT_BASE;
    let hartid = read_mhartid();
    let interval = 1000000; // <- # no. cycles ~ 1/10 sec in qemu.
    write_clint(hartid, clint, interval);
    
    let mut clint = timervec::Clint::new(clint);
    clint.init(hartid as usize, interval);

    // Set the machine trap vector to hold fn ptr to timervec:
    // https://stackoverflow.com/questions/50717928/what-is-the-difference-between-mscratch-and-mtvec-registers
    let timervec_fn = timervec::timervec as *const (); 
    write_mtvec(timervec_fn);
    
    // Enable machine mode interrupts with mstatus reg.
    write_mstatus(read_mstatus() | MSTATUS_MIE);

    // Enable machine-mode timer interrupts.
    write_mie(read_mie() | MIE_MTIE);

}

/// This gets called from src/entry.rs and runs on each hart.
/// The principle goal is to run configuration steps that will
/// allow us to run our kernel in supervisor mode. After this
/// per-hart configuration function runs it calls main(), which
/// is where we bootstrap and init the kernel.
///
/// This is referenced from the xv6-riscv kernel, as we had no
/// knowledge of how to configure riscv h/w. 
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

    // Allow our kernel to handle interrupts from sup mode
    // by "delegating" interrupts and exceptions.
    // medeleg => synchronous interrupt
    // mideleg => asynchronous interrupt
    write_medeleg(0xffff); // Check 3.1.8 in: (haven't read it in full yet)
    write_mideleg(0xffff); // https://five-embeddev.com/riscv-isa-manual/latest/machine.html#machine
    write_sie(read_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);

    // Now give sup mode access to (all??) of phys mem.
    // Check 3.1.6 of line 66 link.
    write_pmpaddr0(0x3fffffffffffff_u64); // Prayers that ULL == u64
    write_pmpcfg0(0xf);

    // Get interrupts from clock, handled by timerinit().
    timerinit();

    // Store each hart's hartid in its tp reg for identification.
    let hartid = read_mhartid();
    write_tp(hartid);

    // Now return to sup mode and jump to main().
    call_mret();

}

// Primary kernel bootstrap function.
// We ensure that we only initialize kernel subsystems
// one time by only doing so on hart0, and sending
// any other hart to essentially wait for interrupt (wfi).
fn main() -> ! {
    // We only bootstrap on hart0.
    let id = riscv::read_tp();
    if id == 0 {
        uart::Uart::init();
        println!("{}", param::BANNER);
        log!(Info, "Bootstrapping on hart0...");
    } else {
    }

    loop {}
}
