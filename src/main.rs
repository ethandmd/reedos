#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod uart;
pub mod entry;
pub mod riscv;

const NHART: usize = 2;

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



// a scratch area per CPU for machine-mode timer interrupts.
// uint64 timer_scratch[NCPU][5];
static mut TIMER_SCRATCH: [[u64; 5]; NHART] = [[0;5]; NHART];

// Have to get the timer interrupts that arrive in mach mode
// and convert to s/w interrupts for trap.
fn timerinit() {
    let hartid = riscv::read_mhartid();
    let interval = 1000000; // <- # no. cycles ~ 1/10 sec in qemu.
    riscv::write_clint(hartid, interval);
    
    // uint64 *scratch = &timer_scratch[id][0];
    unsafe {
        // TIMER_SCRATCH[id][0..2] : let timervec function save registers here.
        // CLINT_MTIMECMP register address for mmio
        TIMER_SCRATCH[hartid as usize][3] = riscv::CLINT_BASE + 0x4000 + 8*(hartid);
        // Interval length in cycles
        TIMER_SCRATCH[hartid as usize][4] = interval;
    }

    let scratch_addr;
    unsafe {
        scratch_addr = TIMER_SCRATCH.as_mut_ptr();
    }
    println!("[INFO]: Scratch_addr: {:?}",scratch_addr);
    riscv::write_mscratch(scratch_addr as u64);

    // set the machine-mode trap handler.
    // write_mtvec((uint64)timervec);

    // enable machine-mode interrupts.
    // write_mstatus(r_mstatus() | MSTATUS_MIE);

    // enable machine-mode timer interrupts.
    // write_mie(r_mie() | MIE_MTIE);

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
    // medeleg => synchronous interrupt
    // mideleg => asynchronous interrupt
    riscv::write_medeleg(0xffff); // Check 3.1.8 in: (haven't read it in full yet)
    riscv::write_mideleg(0xffff); // https://five-embeddev.com/riscv-isa-manual/latest/machine.html#machine
    riscv::write_sie(
        riscv::read_sie() | riscv::SIE_SEIE | riscv::SIE_STIE | riscv::SIE_SSIE
    );

    // Now give sup mode access to (all??) of phys mem.
    // Check 3.1.6 of line 66 link.
    riscv::write_pmpaddr0(0x3fffffffffffff_u64); // Prayers that ULL == u64
    riscv::write_pmpcfg0(0xf);

    // Get interrupts from clock, handled by timerinit().
    timerinit();

    // Store each hart's hartid in its tp reg for identification.
    let hartid = riscv::read_mhartid();
    riscv::write_tp(hartid);

    // Now return to sup mode and jump to main().
    println!("[INFO]: Jumping to main fn (and sup mode)");
    riscv::call_mret();

}

// Doesn't need to be extern C, no_mangle, nothin' fancy...?
fn main() -> ! {
    // Init uart driver.
    let mut uartd = uart::Uart::new(0x1000_0000);
    uartd.init();
    println!("[INFO]: Entered main()");
    // Seasons greetings.
    println!("MELLOW SWIRLED!\n from,\n your fav main fn");
    println!("(called from _start fn!)");

    loop {}
}
