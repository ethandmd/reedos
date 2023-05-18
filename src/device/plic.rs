// PLIC device. There is usually only one. It should be locked probably.

// The PLIC mediates S-Mode and M-Mode external interrupts across all Harts.
// The interface exists at a known memory location: PLIC_BASE

// When a device asks for an interrupt, it provides its priority level.
// If a device's interrupt IRQ-value is above a threshold, the PLIC
// serves its interrupt. Each Hart has a distinct threshold encoded at
// a fixed offset and step from its HartID. Serving interrupts, the PLIC
// device gives you the highest priority pending interrupt.
// The claim function sends a read-signal to an mmapped register.
// The complete function sends a write-signal to the mmapped register.

// REFS: https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc#interrupt-priorities
// https://github.com/Pooletergeist/reedos/blob/40-move-to-qemu-sifive-unleashed/src/device/plic.rs
// https://github.com/sgmarz/osblog/blob/master/risc_v/src/plic.rs (a little)
    // could use it more.. e.g. cases more rustlike.


use core::cell::OnceCell; // for PLIC, write once read many times
use crate::hw::riscv;
use crate::hw::param::{PLIC_BASE, UART_IRQ, VIRTIO_IRQ};

// ^ constants for PLIC_BASE & device interrupt (IRQ) priority locations.

pub static mut PLIC: OnceCell<Plic> = OnceCell::new(); // all memory accesses to Plic go through here!

pub struct Plic {
    base: usize,
}

/// Single-time global initialization for Plic.
/// Sets magic number device IRQ priorities, then initializes.
pub fn global_init() {
    // set desired IRQ priorities non-zero (otherwise disabled).
    // currently just for UART
    let base_addr = PLIC_BASE as *mut u32;

    unsafe {
        base_addr.add(UART_IRQ).write_volatile(1);
        base_addr.add(VIRTIO_IRQ).write_volatile(1);
    }

    // initialize PLIC
    unsafe {
        match PLIC.set(Plic::new(PLIC_BASE)) {
            Ok(()) => {},
            Err(_) => panic!("Plic double init!"),
        }
        assert!(PLIC.get().is_some());
    }
}

/// Local initialize Plic. Once per HART. Enables devices, sets threshold.
pub fn local_init() {

    //  set enable bits for this hart's S-mode
    let bit_mask: u32 = (1 << UART_IRQ) | (1 << VIRTIO_IRQ);

    unsafe {
        // call the write to Plic magic locations for the enabled bits.
        assert!(PLIC.get().is_some());
        PLIC.get().expect("PLIC Once Cell is not written").hart_local_enable(bit_mask);

        // accept interrupts from all enabled devices with priority > 0.
        PLIC.get().expect("PLIC should be initialized").set_s_priority_threshold(0);
    }
}

// currently stolen directly from xv6-riscv

// TODO these should really be mut calls, since they have potential to
// change internal state. However they are not so we can use a
// OnceCell. Consider using MaybeUninit, or something else. A mutex is
// not the answer, as PLICs can and should be used in parallel by
// multiple harts.

/// new makes Plic at a specific place with specific interrupts enabled.
impl Plic {
    fn new(base: usize) -> Self {
        // new Plic at base
        let out = Plic {
            base,
        };

        out
    }

    /// take a Plic and a threshold, set threshold at a magic location
    /// known explicitly per hart: (RAW_OFFSET + hart * RAW_STEP).
    fn set_s_priority_threshold(&self, threshold: u32) {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201000;
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP))/4; // / 4?
        unsafe {
            addr.add(final_offset).write_volatile(threshold);
        }
    }

    /// enables interrupts from devices by non-zero bits in bit-mask
    pub fn hart_local_enable(&self, bit_mask: u32) {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x2080;
        const RAW_STEP: usize = 0x100;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP))/4; //ASK: /4?

        unsafe {
            addr.add(final_offset).write_volatile(bit_mask); //ASK: length?
        }
    }

    /// Claim an interupt that you were alerted to.
    pub fn claim(&self) -> u32 {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201004; // 4-bits after threshold
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP))/4; //ASK / 4?
        unsafe {
            // returns highest-priority pending interrupt
            addr.add(final_offset).read_volatile()
            // ^ reading mmapped register
        }
    }

    /// Alert the PLIC that we have completed the interupt we claimed
    pub fn complete(&self, irq: u32) {
        let addr = self.base as *mut u32;
        let hart = riscv::read_tp() as usize;

        const RAW_OFFSET: usize = 0x201004;
        const RAW_STEP: usize = 0x2000;

        let final_offset = (RAW_OFFSET + (hart * RAW_STEP))/4; //ASK: / 4?
        unsafe {
            // signals completion of interrupt identified by IRQ
            addr.add(final_offset).write_volatile(irq);
            // ^ writing mmapped register
        }
    }
}
