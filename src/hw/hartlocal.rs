//! This is the module for the initial information restored to a hart
//! when exiting a process back into the kernel

use core::arch::asm;
use alloc::boxed::Box;

use crate::process::Process;
use crate::hw::riscv::{write_gp, read_gp};

/// What do we need to restore when returning from a process
pub struct GPInfo {
    pub current_process: Process,
    // TODO consider moving the page table and the sp from the
    // sscratch stack to here
    //
    // Currently we aren't doing that becuase we need(?) that info to
    // boostrap this, which has stronger requirements about playing
    // nice with rust
}

impl GPInfo {
    pub fn new(current_process: Process) -> Self {
        Self {
            current_process,
        }
    }
}

/// "Consumes" the global pointer info (most importantly the process)
/// from the rust persective, while placing a reference to it into gp
/// for later use
pub fn save_gp_info64(gpi: GPInfo) {
    let ptr = Box::into_raw(Box::new(gpi));
    // ^ If I understand correctly, this should consume gpi and give
    // me a pointer to a heap allocated version of gpi that will
    // outlive this function and can safely be referenced from gp
    // later

    write_gp(ptr as u64);
}

pub fn restore_gp_info64() -> GPInfo {
    let ptr = read_gp() as *mut GPInfo;
    unsafe {
        let b_ptr = Box::from_raw(ptr);

        Box::into_inner(b_ptr)
    }
}

pub fn hartlocal_info_interrupt_stack_init() {
    let gpi = GPInfo {
        current_process: Process::new_uninit(),
    };
    save_gp_info64(gpi);
    unsafe {
        asm!(
            "csrr a0, sscratch",
            "addi a0, a0, -8",
            "sd gp, (a0)",
            "csrw sscratch, a0",
            out("a0") _      // clobbers
        )
    }
}
