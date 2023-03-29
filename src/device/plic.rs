// PLIC device. There is usually only one. It should be locked probably.
use core::mem::{size_of, MaybeUninit};

use crate::hw::riscv;
use crate::lock::mutex::*;
use crate::hw::param::PLIC_BASE;

pub static mut PLIC: MaybeUninit<Mutex<Plic>> = MaybeUninit::uninit();

pub struct Plic {
    base: usize,
}

/// Single time global initialization. Takes a bit mask of the
/// interupts to enable through the plic by index.
pub fn global_init(ints: usize) {
    unsafe {
        PLIC.write(Mutex::new(Plic::new(PLIC_BASE, ints)));
    }
}

// currently stolen directly from xv6-riscv
impl Plic {
    fn new(base: usize, ints: usize) -> Self {
        let out = Plic {
            base,
        };

        let addr = out.base as *mut u32;

        for int in 0..size_of::<usize>() {
            if (ints >> int) & 1 != 0 {
                // we need to enable this interupt
                // set its priority
                unsafe {
                    *addr.offset(int as isize) = 1; // priority
                }
            }
        }

        out
    }

    fn set_s_priority_threshold(&mut self, threshold: u32) {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201000;
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP)) / 4;
        unsafe {
            addr.offset(final_offset as isize).write_volatile(threshold);
        }
    }

    pub fn hart_local_init(&mut self) {
        self.set_s_priority_threshold(0); // accept everything
    }

    /// Claim an interupt that you were alerted to.
    pub fn claim(&mut self) -> u32 {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201004;
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP)) / 4;
        unsafe {
            addr.offset(final_offset as isize).read_volatile()
        }
    }

    /// Alert the PLIC that we have completed the interupt we claimed
    pub fn complete(&mut self, irq: u32) {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201004;
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP)) / 4;
        unsafe {
            addr.offset(final_offset as isize).write_volatile(irq);
        }
    }
}
