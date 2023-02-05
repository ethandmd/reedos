#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod uart;
pub mod entry;
pub mod riscv;

#[macro_export]
macro_rules! print
{
    ($($args:tt)+) => ({
            use core::fmt::Write;
            let _ = write!(uart::Uart::new(0x1000_0000), $($args)+);
            });
}

#[macro_export]
macro_rules! println
{
    () => ({
           print!("\r\n")
           });
    ($fmt:expr) => ({
            print!(concat!($fmt, "\r\n"))
            });
    ($fmt:expr, $($args:tt)+) => ({
            print!(concat!($fmt, "\r\n"), $($args)+)
            });
}

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Have to get the timer interrupts that arrive in mach mode
// and convert to s/w interrupts for trap.
fn timerinit() {
//    let hartid = riscv::read_mhartid();
//    let interval = 1000000; // <- # no. cycles ~ 1/10 sec in qemu.
//    let mtcmp = riscv::clint_mtimecmp(hartid);
    // TODO
}

// Referenced from xv6-riscv/kernel/start.c:
// Here, we are going to perform set up in machine mode.
// However, we are going to return (to main) via mret (riscv).
#[no_mangle]
pub extern "C" fn _start() {
    let fn_main = main as *const ();

    let mut uartd = uart::Uart::new(0x1000_0000);
    uartd.init();
    println!("[INFO]: Currently on hartid: {}", riscv::read_mhartid());
   
    // Set the *prior* privilege mode to supervisor.
    // Bits 12, 11 are for MPP. They are WPRI.
    // For sstatus we can write SPP reg, bit 8.
    let mut ms = riscv::read_mstatus();
    ms &= !riscv::MSTATUS_MPP_MASK;
    ms |= riscv::MSTATUS_MPP_S; 
    riscv::write_mstatus(ms);

    // Set machine exception prog counter to 
    // our main function for later mret call.
    println!("[INFO]: main fn's addr?: {:?}", fn_main);
    riscv::write_mepc(fn_main);

    // Disable paging while setting up.
    riscv::write_satp(0);

    // Allow our kernel to handle interrupts from sup mode
    // by "delegating" interrupts and exceptions.
    riscv::write_medeleg(0xffff); // Check 3.1.8 in: (haven't read it in full yet)
    riscv::write_mideleg(0xffff); // https://five-embeddev.com/riscv-isa-manual/latest/machine.html#machine
    riscv::write_sie(
        riscv::read_sie() | riscv::SIE_SEIE | riscv::SIE_STIE | riscv::SIE_SSIE
    );

    // Now give sup mode access to (all??) of phys mem.
    // Check 3.1.6 of line 66 link.
    riscv::write_pmpaddr0(0x3fffffffffffff as u64); // Prayers that ULL == u64
    riscv::write_pmpcfg0(0xf);

    // Get interrupts from clock, handled by timerinit().
    timerinit();

    // Store each hart's hartid in its tp reg for identification.
    let hartid = riscv::read_mhartid();
    riscv::write_tp(hartid);

    // Now return to sup mode and jump to main().
    riscv::call_mret();

}

// Doesn't need to be extern C, no_mangle, nothin' fancy...?
fn main() -> ! {
    // Init uart driver.
    let mut uartd = uart::Uart::new(0x1000_0000);
    uartd.init();
    // Seasons greetings.
    println!("MELLOW SWIRLED!\n from,\n your fav main fn");
    println!("(called from _start fn!)");

    loop {}
}
