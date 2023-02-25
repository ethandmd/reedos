use core::arch::global_asm;
//use crate::device::uart;
use crate::device::clint;
use crate::hw::riscv;

use crate::log;

// TODO currently we are using machine mode direct for everything. So everything goes though mhandler

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}

pub fn init() {
    riscv::write_stvec(__strapvec as usize);
}

fn s_handler() {
    let cause = riscv::read_scause();

    match cause {
        _ => {
            // log::log!(Warning, "Uncaught supervisor mode interupt. scause: {:X}", cause);
            panic!()
        }
    }

    // riscv::write_sip(0);
}

fn m_handler() {
    let mcause = riscv::read_mcause();

    match mcause {
        riscv::MSTATUS_TIMER => {
            // log::log!(Debug, "Machine timer interupt, hart: {}", riscv::read_mhartid());
            clint::set_mtimecmp(10_000_000);
        },
        _ => {
            log::log!(Warning, "Uncaught machine mode interupt. mcause: {:X}", mcause);
            // panic!();
        }
    }
}

//     li a1, 2
// csrw sip, a1
global_asm!(
    r#"
    .option norvc
    .align 4
    .global __mtrapvec
__mtrapvec:
    call {}

    mret
    "#, sym m_handler
    );

global_asm!(
    r#"
    .option norvc
    .align 4
    .globl __strapvec
 __strapvec:
    addi sp, sp, -256
  
    sd x0, 0(sp)
    sd x1, 8(sp)
    sd x2, 16(sp)
    sd x3, 24(sp)
    sd x4, 32(sp)
    sd x5, 40(sp)
    sd x6, 48(sp)
    sd x7, 56(sp)
    sd x8, 64(sp)
    sd x9, 72(sp)
    sd x10, 80(sp)
    sd x11, 88(sp)
    sd x12, 96(sp)
    sd x13, 104(sp)
    sd x14, 112(sp)
    sd x15, 120(sp)
    sd x16, 128(sp)
    sd x17, 136(sp)
    sd x18, 144(sp)
    sd x19, 152(sp)
    sd x20, 160(sp)
    sd x21, 168(sp)
    sd x22, 176(sp)
    sd x23, 184(sp)
    sd x24, 192(sp)
    sd x25, 200(sp)
    sd x26, 208(sp)
    sd x27, 216(sp)
    sd x28, 224(sp)
    sd x29, 232(sp)
    sd x30, 240(sp)

    call {}
 
    ld x0, 0(sp)
    ld x1, 8(sp)
    ld x2, 16(sp)
    ld x3, 24(sp)
    ld x5, 40(sp)
    ld x6, 48(sp)
    ld x7, 56(sp)
    ld x8, 64(sp)
    ld x9, 72(sp)
    ld x10, 80(sp)
    ld x11, 88(sp)
    ld x12, 96(sp)
    ld x13, 104(sp)
    ld x14, 112(sp)
    ld x15, 120(sp)
    ld x16, 128(sp)
    ld x17, 136(sp)
    ld x18, 144(sp)
    ld x19, 152(sp)
    ld x20, 160(sp)
    ld x21, 168(sp)
    ld x22, 176(sp)
    ld x23, 184(sp)
    ld x24, 192(sp)
    ld x25, 200(sp)
    ld x26, 208(sp)
    ld x27, 216(sp)
    ld x28, 224(sp)
    ld x29, 232(sp)
    ld x30, 240(sp)
  
    addi sp, sp, 256
    sret
    "#, sym s_handler
    );
