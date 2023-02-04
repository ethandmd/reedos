#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod uart;

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

#[no_mangle]
pub extern "C" fn _start() -> ! {

    let mut myuart = uart::Uart::new(0x1000_0000);
    myuart.init();

    println!("MELLOW SWIRLED!");

    loop {}
}
