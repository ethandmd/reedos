//! System parameters and memory layout.
//
// Qemu riscv sifive_u machine: https://github.com/qemu/qemu/blob/master/hw/riscv/sifive_u.c
// Devices: UART, CLINT, PLIC, PRCI, GPIO, OTP, GEM (ethernet), DMA, SPIO, SPI2, PWM{0,1}
// /* CLINT timebase frequency */
// #define CLINT_TIMEBASE_FREQ 1000000

//static const MemMapEntry sifive_u_memmap[] = {
//    [SIFIVE_U_DEV_DEBUG] =    {        0x0,      0x100 },
//    [SIFIVE_U_DEV_MROM] =     {     0x1000,     0xf000 },
//    [SIFIVE_U_DEV_CLINT] =    {  0x2000000,    0x10000 },
//    [SIFIVE_U_DEV_L2CC] =     {  0x2010000,     0x1000 },
//    [SIFIVE_U_DEV_PDMA] =     {  0x3000000,   0x100000 },
//    [SIFIVE_U_DEV_L2LIM] =    {  0x8000000,  0x2000000 },
//    [SIFIVE_U_DEV_PLIC] =     {  0xc000000,  0x4000000 },
//    [SIFIVE_U_DEV_PRCI] =     { 0x10000000,     0x1000 },
//    [SIFIVE_U_DEV_UART0] =    { 0x10010000,     0x1000 },
//    [SIFIVE_U_DEV_UART1] =    { 0x10011000,     0x1000 },
//    [SIFIVE_U_DEV_PWM0] =     { 0x10020000,     0x1000 },
//    [SIFIVE_U_DEV_PWM1] =     { 0x10021000,     0x1000 },
//    [SIFIVE_U_DEV_QSPI0] =    { 0x10040000,     0x1000 },
//    [SIFIVE_U_DEV_QSPI2] =    { 0x10050000,     0x1000 },
//    [SIFIVE_U_DEV_GPIO] =     { 0x10060000,     0x1000 },
//    [SIFIVE_U_DEV_OTP] =      { 0x10070000,     0x1000 },
//    [SIFIVE_U_DEV_GEM] =      { 0x10090000,     0x2000 },
//    [SIFIVE_U_DEV_GEM_MGMT] = { 0x100a0000,     0x1000 },
//    [SIFIVE_U_DEV_DMC] =      { 0x100b0000,    0x10000 },
//    [SIFIVE_U_DEV_FLASH0] =   { 0x20000000, 0x10000000 },
//    [SIFIVE_U_DEV_DRAM] =     { 0x80000000,        0x0 },
//};

use core::ptr::addr_of_mut;


/// CLINT base address.
pub const CLINT_BASE: usize = 0x2000000;

/// UART0 base adderss.
pub const UART0_BASE: usize = 0x10010000;

/// PLIC base address.
pub const PLIC_BASE: usize = 0xc000000;

/// Start of kernel memory (first .text section goes here).
pub const DRAM_BASE: *mut usize = 0x80000000 as *mut usize;

// NOTE:
// We can't just use link_name for linker symbols, cause they don't
// bind correctly for some reason.
// Instead, use core::ptr::addr_of!() to get address and then cast to usize.
//
// TODO consider reworking this to have a consistent naming scheme and
// maybe a macro for the getter functions.
extern "C" {
    static mut _text_end: usize;
    static mut _bss_start: usize;
    static mut _bss_end: usize;
    static mut _memory_end: usize;
    static mut _roedata: usize;
    static mut _edata: usize;
    static mut _stacks_start: usize;
    static mut _stacks_end: usize;
    static mut _intstacks_start: usize;
    static mut _intstacks_end: usize;
}
pub fn text_end() -> *mut usize {
    unsafe { addr_of_mut!(_text_end) }
}

pub fn bss_end() -> *mut usize {
    unsafe { addr_of_mut!(_bss_end) }
}

pub fn bss_start() -> *mut usize {
    unsafe { addr_of_mut!(_bss_start) }
}

pub fn rodata_end() -> *mut usize {
    unsafe { addr_of_mut!(_roedata) }
}

pub fn data_end() -> *mut usize {
    unsafe { addr_of_mut!(_edata) }
}

pub fn stacks_start() -> *mut usize {
    unsafe { addr_of_mut!(_stacks_start) }
}

pub fn stacks_end() -> *mut usize {
    unsafe { addr_of_mut!(_stacks_end) }
}

pub fn intstacks_start() -> *mut usize {
    unsafe { addr_of_mut!(_intstacks_start) }
}

pub fn intstacks_end() -> *mut usize {
    unsafe { addr_of_mut!(_intstacks_end) }
}

pub fn dram_end() -> *mut usize {
    unsafe { addr_of_mut!(_memory_end) }
}

pub static PAGE_SIZE: usize = 4096;

// Run parameters
pub const NHART: usize = 2;

// Unnecessary.
pub static BANNER: &str = r#"
Mellow Swirled,
                       __
   ________  ___  ____/ /___  _____
  / ___/ _ \/ _ \/ __  / __ \/ ___/
 / /  /  __/  __/ /_/ / /_/ (__  )
/_/   \___/\___/\__,_/\____/____/

"#;
