use crate::hw::riscv;
use crate::device::clint;

use crate::log;

// TODO currently we are using machine mode direct for everything. So
// everything goes though mhandler. The situation has become more
// complicated, see the issue on github about it.

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}

pub fn init() {
    riscv::write_stvec(__strapvec as usize);
}

#[no_mangle] pub extern "C" fn m_handler() {
    let mcause = riscv::read_mcause();

    match mcause {
        riscv::MSTATUS_TIMER => {
            // log::log!(Debug, "Machine timer interupt, hart: {}", riscv::read_mhartid());
            clint::set_mtimecmp(10_000_000);
        },
        _ => {
            log::log!(Warning, "Uncaught machine mode interupt. mcause: {:X}", mcause);
            panic!();
        }
    }
}

#[no_mangle] pub extern "C" fn s_handler() {
    let cause = riscv::read_scause();

    match cause {
        _ => {
            log::log!(Warning, "Uncaught supervisor mode interupt. scause: {:X}", cause);
            panic!()
        }
    }
}
