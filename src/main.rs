#![no_std]
#![no_main]

use core::panic::PanicInfo;

// The never type "!" means diverging function (never returns).
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static HELLO: &[u8] = b"Hello World!";

// Kernel entry point.
// Zero out bss section. Init data section and get to
// our entry point.

#[link_section = ".init.rust"]
#[export_name = "_start_rust"]
pub extern "C" fn _start() -> ! {
    let vga_buffer = 0xb8000 as *mut u8;

    for (i, &byte) in HELLO.iter().enumerate() {
        unsafe {
            *vga_buffer.offset(i as isize * 2) = byte;
            *vga_buffer.offset(i as isize * 2 + 1) = 0xb;
        }
    }

    loop {}
}
