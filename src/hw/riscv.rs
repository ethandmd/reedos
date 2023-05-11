//! Rust wrappers around RISC-V routines
use core::arch::asm;

/// Machine previous protection mode.
pub const MSTATUS_MPP_MASK: u64 = 3 << 11; // Mask for bit tricks
pub const MSTATUS_MPP_M: u64 = 3 << 11; // Machine
pub const MSTATUS_MPP_S: u64 = 1 << 11; // Supervisor
pub const MSTATUS_MPP_U: u64 = 0 << 11; // User
pub const MSTATUS_MIE: u64 = 1 << 3; // machine-mode interrupt enable.
pub const MSTATUS_TIMER: u64 = (1 << 63) | (7); // mcause for machine mode timer.
                                                // sstatus := Supervisor status reg.
pub const SSTATUS_SUM: u64 = 1 << 18; // Previous mode, 1=Supervisor, 0=User
pub const SSTATUS_SPP: u64 = 1 << 8; // Previous mode, 1=Supervisor, 0=User
pub const SSTATUS_SPIE: u64 = 1 << 5; // Supervisor Previous Interrupt Enable
pub const SSTATUS_UPIE: u64 = 1 << 4; // User Previous Interrupt Enable
pub const SSTATUS_SIE: u64 = 1 << 1; // Supervisor Interrupt Enable
pub const SSTATUS_UIE: u64 = 1 << 0; // User Interrupt Enable

/// Machine-mode Interrupt Enable
pub const MIE_MEIE: u64 = 1 << 11; // external
pub const MIE_MTIE: u64 = 1 << 7; // timer
pub const MIE_MSIE: u64 = 1 << 3; // software

/// Supervisor Interrupt Enable
pub const SIE_SEIE: u64 = 1 << 9; // external
pub const SIE_STIE: u64 = 1 << 5; // timer
pub const SIE_SSIE: u64 = 1 << 1; // software

/// Return id of current hart while in machine mode.
pub fn read_mhartid() -> u64 {
    let id: u64;
    // Volatile by default?
    unsafe {
        asm!("csrr {}, mhartid", out(reg) id);
    }
    id
}

/// Read CSR := Control and Status Register mstatus.
/// Refer to chap 9 of riscv isa manual for info on CSRs.
pub fn read_mstatus() -> u64 {
    let status: u64;
    unsafe {
        asm!("csrr {}, mstatus", out(reg) status);
    }
    status
}

// Write to mstatus.
pub fn write_mstatus(status: u64) {
    unsafe {
        asm!("csrw mstatus, {}", in(reg) status);
    }
}

/// Read the integer encoded 'reason' why a trap was called (machine mode).
pub fn read_mcause() -> u64 {
    let cause: u64;
    unsafe {
        asm!("csrr {}, mcause", out(reg) cause);
    }
    cause
}

/// Read the integer encoded 'reason' why a trap was called (supervisor mode).
pub fn read_scause() -> u64 {
    let cause: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) cause);
    }
    cause
}

/// Set mepc := machine exception program counter.
/// (what instr (address) to go to from exception.)
pub fn write_mepc(addr: *const ()) {
    unsafe {
        asm!("csrw mepc, {}", in(reg) addr);
    }
}

pub fn read_mepc() -> usize {
    let addr: usize;
    unsafe {
        asm!("csrr {}, mepc", out(reg) addr);
    }
    addr
}

pub fn read_sstatus() -> u64 {
    let status: u64;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) status);
    }
    status
}

pub fn write_status(status: u64) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) status);
    }
}

// Enable sup mode interrupt and exception.
pub fn read_sip() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, sip", out(reg) x);
    }
    x
}

pub fn write_sip(ire: u64) {
    unsafe {
        asm!("csrw sip, {}", in(reg) ire);
    }
}

pub fn read_sie() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, sie", out(reg) x);
    }
    x
}

pub fn write_sie(ire: u64) {
    unsafe {
        asm!("csrw sie, {}", in(reg) ire);
    }
}

pub fn read_mie() -> u64 {
    let x: u64;
    unsafe {
        asm!("csrr {}, mie", out(reg) x);
    }
    x
}

pub fn write_mie(x: u64) {
    unsafe {
        asm!("csrw mie, {}", in(reg) x);
    }
}

/// SATP Sv39 mode: (8L << 60)
// From addr to satp reg: (pagetable) (SATP_SV39 | (((uint64)pagetable) >> 12))
pub fn read_satp() -> usize {
    let pt: usize;
    unsafe {
        asm!("csrr {}, satp", out(reg) pt);
    }
    pt
}

pub fn write_satp(pt: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) pt);
    }
}

/// medeleg := machine exception delegation (to supervisor mode)
pub fn read_medeleg() -> u64 {
    let med: u64;
    unsafe {
        asm!("csrr {}, medeleg", out(reg) med);
    }
    med
}

pub fn write_medeleg(med: u64) {
    unsafe {
        asm!("csrw medeleg, {}", in(reg) med);
    }
}

/// mideleg := machine interrupt delegation (to supervisor mode)
pub fn read_mideleg() -> u64 {
    let mid: u64;
    unsafe {
        asm!("csrr {}, mideleg", out(reg) mid);
    }
    mid
}

pub fn write_mideleg(mid: u64) {
    unsafe {
        asm!("csrw mideleg, {}", in(reg) mid);
    }
}

/// pmpaddr := phys mem protection addr.
/// Configure to give supervisor mode access to
/// certain parts of memory.
pub fn write_pmpaddr0(addr: u64) {
    unsafe {
        asm!("csrw pmpaddr0, {}", in(reg) addr);
    }
}

pub fn read_pmpaddr0() -> usize {
    let addr: usize;
    unsafe {
        asm!("csrr {}, pmpaddr0", out(reg) addr);
    }
    addr
}

pub fn write_pmpcfg0(addr: u64) {
    unsafe {
        asm!("csrw pmpcfg0, {}", in(reg) addr);
    }
}

pub fn read_pmpcfg0() -> usize {
    let addr: usize;
    unsafe {
        asm!("csrr {}, pmpcfg0", out(reg) addr);
    }
    addr
}

/// Just for curiosity's sake:
/// <https://github.com/rust-lang/rust/issues/82753>
/// tp := thread pointer register.
/// This way we can query a hart's hartid and store it in tp reg.
pub fn write_tp(id: u64) {
    unsafe {
        asm!("mv tp, {}", in(reg) id);
    }
}

pub fn read_tp() -> u64 {
    let tp: u64;
    unsafe {
        asm!("mv {}, tp", out(reg) tp);
    }
    tp
}

/// Read and write the hart local global pointer register. In kernel
/// space we will be using it to point to hart local kernel
/// information including the current process to be / has been run
pub fn write_gp(id: u64) {
    unsafe {
        asm!("mv gp, {}", in(reg) id);
    }
}

pub fn read_gp() -> u64 {
    let gp: u64;
    unsafe {
        asm!("mv {}, gp", out(reg) gp);
    }
    gp
}

// Make sure mret has an addr to go to!
pub fn call_mret() {
    unsafe {
        asm!("mret");
    }
}

pub fn write_mscratch(scratch: usize) {
    unsafe {
        asm!("csrw mscratch, {}", in(reg) scratch);
    }
}

pub fn write_mtvec(addr: usize) {
    unsafe {
        asm!(r#"
        .option norvc
        csrw mtvec, {}
        "#, in(reg) addr);
    }
}

pub fn read_mtvec() -> usize {
    let addr: usize;
    unsafe {
        asm!("csrr {}, mtvec", out(reg) addr);
    }
    addr
}

pub fn write_stvec(addr: usize) {
    unsafe {
        asm!(r#"
        .option norvc
        csrw stvec, {}
        "#, in(reg) addr);
    }
}

pub fn read_stvec() -> usize {
    let addr: usize;
    unsafe {
        asm!("csrr {}, stvec", out(reg) addr);
    }
    addr
}

/// The `zero, zero` arguments to `sfence.vma` insn mean
/// we completely flush every TLB entry for all ASIDs.
pub fn flush_tlb() {
    unsafe {
        asm!("sfence.vma zero, zero");
    }
}

// Riscv unprivileged spec A.4.2: I/O Ordering
pub fn io_barrier() {
    unsafe { asm!("fence w,o"); }
}
