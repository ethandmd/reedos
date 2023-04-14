//! Kernel trap handlers.
use crate::device::clint;
use crate::hw::riscv;

use crate::log;

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}


// TODO currently not in use
//
// pub struct TrapFrame {
//     kpgtbl: *mut PageTable,
//     handler: *const (),
//     cause: usize,
//     retpc: usize, // Return from trap program counter value.
//     regs: [usize; 32],
// }

/// Write the supervisor trap vector to stvec register on each hart.
pub fn init() {
    riscv::write_stvec(__strapvec as usize);
}

/// Machine mode trap handler.
#[no_mangle]
pub extern "C" fn m_handler() {
    let mcause = riscv::read_mcause();

    match mcause {
        riscv::MSTATUS_TIMER => {
            // log::log!(Debug, "Machine timer interupt, hart: {}", riscv::read_mhartid());
            clint::set_mtimecmp(10_000_000);
        }
        _ => {
            log::log!(
                Warning,
                "Uncaught machine mode interupt. mcause: 0x{:x}",
                mcause
            );
            panic!();
        }
    }
}

/// Supervisor mode trap handler.
#[no_mangle]
pub extern "C" fn s_handler() {
    let cause = riscv::read_scause();

    {
        log::log!(
            Warning,
            "Uncaught supervisor mode interupt. scause: 0x{:x}",
            cause
        );
        panic!()
    }
}

//--------------------------------------------------------------------
//
// After this is late boot stuff

/// System call rust handler. This is called from scall_asm. See there
/// for calling convention info.
///
/// Currently this is run in supervisior mode, on the program stack,
/// executing the kernel text. this means the non-static globals are
/// not available, as is any piece of memory not mapped in either the
/// program or in kernel text.
///
/// This may change in the future.
#[no_mangle]
pub extern "C" fn scall_rust(a0: usize, a1: usize, a2: usize, a3: usize,
                             a4: usize, a5: usize, a6: usize, a7: usize) {
    match a7 {
        _ => {
            panic!("Uncaught system call: {}", a7);
        }
    }
}
