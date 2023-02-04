#![no_std]
#![no_main]

use core::panic::PanicInfo;

use core::arch::global_asm;

pub mod uart;

#[macro_export]
macro_rules! print
{
	($($args:tt)+) => ({
			use core::fmt::Write;
			let _ = write!(crate::uart::Uart::new(0x1000_0000), $($args)+);
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

// qemu -kernel loads the kernel at 0x80000000
// and causes each hart (i.e. CPU) to jump there.
// kernel.ld causes the following code to
// be placed at 0x80000000.

// set up a stack for C.
// stack0 is declared in start.c,
// with a 4096-byte stack per CPU.
// sp = stack0 + (hartid * 4096)

// # jump to start() in start.c

// global_asm!(
    // ".section .data",
	    // ".global _stack0",
	    // "_stack0:",
	    // ".int 0x9000000",
	    // ".section .text",
// 	    "	.global _entry",
// 	    "	.extern _start",
// 	    "	_entry:",
// 	    "    la sp, _stack0",
// 	    "        li a0, 1024*4",
// 	    "        csrr a1, mhartid",
// 	    "        addi a1, a1, 1",
// 	    "        mul a0, a0, a1",
// 	    "        add sp, sp, a0",
// 	    "    call _start",
// 	    "	spin:",
// 	    "    j spin"
// );
// global_asm!(include_str!("entry.S"));

#[no_mangle]
pub extern "C" fn _start() -> ! {

    let mut myuart = uart::Uart::new(0x1000_0000);
    myuart.init();

    println!("MELLOW SWIRLED!");

    loop {}
}
