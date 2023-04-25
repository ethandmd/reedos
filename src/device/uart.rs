//! Driver code for UART MM I/O device
// Referenced from:
// https://github.com/mit-pdos/xv6-riscv/blob/riscv/kernel/uart.c
// from https://github.com/sgmarz/osblog/tree/master/risc_v/src
use core::fmt::Error;
use core::fmt::Write;

use crate::hw::param::UART_BASE;
use crate::lock::mutex::*;

const IER: usize = 1; // Interrupt Enable Register
const LCR: usize = 3; // Line Control Register (baud rate stuff)
const FCR: usize = 2; // FIFO Control Register (see uart layout in reference)
                      //const LSR: usize = 2; // Line Status Register (ready to rx, ready to tx signals)

pub static mut WRITER: Mutex<Uart> = Uart::new(UART_BASE);

pub struct Uart {
    base_address: usize,
}

impl Write for Uart {
    fn write_str(&mut self, out: &str) -> Result<(), Error> {
        for c in out.bytes() {
            self.put(c);
        }
        Ok(())
    }
}

pub fn init() {
    unsafe {
        WRITER.lock().init();
    }
}

impl Uart {
    pub fn init(&mut self) {
        // https://mth.st/blog/riscv-qemu/AN-491.pdf <-- inclues 16650A ref
        let ptr = self.base_address as *mut u8;
        // Basic semantics:
        // `ptr` is a memory address.
        // We want to write certain values to 'registers' located
        // at specific offsets, calculated by ptr + register_offset.
        // Then, we perform volatile writes to that location in memory
        // to configure the specific parameters of the Qemu virt machine
        // uart device without altering our base address.
        unsafe {
            // Disable interrupts first.
            ptr.add(IER).write_volatile(0x0);
            // Mode in order to set baud rate.
            ptr.add(LCR).write_volatile(1 << 7);
            // baud rate of 38.4k
            ptr.add(0).write_volatile(0x03); // LSB (tx side)
            ptr.add(1).write_volatile(0x00); // MST (rx side)
            // 8 bit words (no parity)
            ptr.add(LCR).write_volatile(3);
            // Enable and clear FIFO
            ptr.add(FCR).write_volatile(1 << 0 | 3 << 1);
            // Enable tx and rx interrupts
            ptr.add(IER).write_volatile(1 << 1 | 1 << 0);
        }
    }

    pub const fn new(base: usize) -> Mutex<Self> {
        let device = Self {
            base_address: base
        };
        Mutex::new(device)
    }

    pub fn put(&mut self, c: u8) {
        let ptr = self.base_address as *mut u8;
        unsafe {
            ptr.add(0).write_volatile(c);
        }
    }

    pub fn get(&mut self) -> Option<u8> {
        let ptr = self.base_address as *mut u8;
        unsafe {
            if ptr.add(5).read_volatile() & 1 == 0 {
                // The DR bit is 0, meaning no data
                None
            } else {
                // The DR bit is 1, meaning data!
                Some(ptr.add(0).read_volatile())
            }
        }
    }
}
