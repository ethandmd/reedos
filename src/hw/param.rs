//! System parameters and memory layout.
// Qemu riscv virt machine memory locations you want to know:
// https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c
//
//static static MemMapEntry virt_memmap[] = {
//    [VIRT_DEBUG] =        {        0x0,         0x100 },
//    [VIRT_MROM] =         {     0x1000,        0xf000 },
//    [VIRT_TEST] =         {   0x100000,        0x1000 },
//    [VIRT_RTC] =          {   0x101000,        0x1000 },
//    [VIRT_CLINT] =        {  0x2000000,       0x10000 },
//    [VIRT_ACLINT_SSWI] =  {  0x2F00000,        0x4000 },
//    [VIRT_PCIE_PIO] =     {  0x3000000,       0x10000 },
//    [VIRT_PLATFORM_BUS] = {  0x4000000,     0x2000000 },
//    [VIRT_PLIC] =         {  0xc000000, VIRT_PLIC_SIZE(VIRT_CPUS_MAX * 2) },
//    [VIRT_APLIC_M] =      {  0xc000000, APLIC_SIZE(VIRT_CPUS_MAX) },
//    [VIRT_APLIC_S] =      {  0xd000000, APLIC_SIZE(VIRT_CPUS_MAX) },
//    [VIRT_UART0] =        { 0x10000000,         0x100 },
//    [VIRT_VIRTIO] =       { 0x10001000,        0x1000 },
//    [VIRT_FW_CFG] =       { 0x10100000,          0x18 },
//    [VIRT_FLASH] =        { 0x20000000,     0x4000000 },
//    [VIRT_IMSIC_M] =      { 0x24000000, VIRT_IMSIC_MAX_SIZE },
//    [VIRT_IMSIC_S] =      { 0x28000000, VIRT_IMSIC_MAX_SIZE },
//    [VIRT_PCIE_ECAM] =    { 0x30000000,    0x10000000 },
//    [VIRT_PCIE_MMIO] =    { 0x40000000,    0x40000000 },
//    [VIRT_DRAM] =         { 0x80000000,           0x0 },
//}

use core::ptr::addr_of_mut;

/// CLINT base address.
pub const CLINT_BASE: usize = 0x2000000;

/// PLIC base address.
pub const PLIC_BASE: usize = 0xc000000;

/// PLIC size in memory
pub const PLIC_SIZE: usize = 0x400000;
//TODO this should be a function of NHART

/// UART base adderss.
pub const UART_BASE: usize = 0x10000000;

/// UART interrupt request number.
pub const UART_IRQ: usize = 10;

/// VIRTIO base address.
pub const VIRTIO_BASE:usize = 0x10001000;

/// VIRTIO size.
pub const VIRTIO_SIZE: usize = 0x4000;

/// VIRTIO interrupt request number.
pub const VIRTIO_IRQ: usize = 1;

/// Start of kernel memory (first .text section goes here).
pub const DRAM_BASE: *mut usize = 0x80000000 as *mut usize;


macro_rules! linker_var {
    (
        $linker_name: ident,
        $rust_name: ident
    ) => {
        extern "C" { static mut $linker_name: usize; }
        #[doc="Get the associated linker variable as a pointer"]
        pub fn $rust_name() -> *mut usize {
            unsafe { addr_of_mut!($linker_name) }
        }
    }
}

// linker_var!(_trampoline_start, trampoline_start);
// linker_var!(_trampoline_end, trampoline_end);
// linker_var!(_trampoline_target, trampoline_target);

linker_var!(_text_start, text_start);
linker_var!(_text_end, text_end);

linker_var!(_bss_start, bss_start);
linker_var!(_bss_end, bss_end);

linker_var!(_rodata_start, rodata_start);
linker_var!(_rodata_end, rodata_end);

linker_var!(_data_start, data_start);
linker_var!(_data_end, data_end);

linker_var!(_stacks_start, stacks_start);
linker_var!(_stacks_end, stacks_end);

linker_var!(_intstacks_start, intstacks_start);
linker_var!(_intstacks_end, intstacks_end);

linker_var!(_memory_end, memory_end);

linker_var!(_global_pointer, global_pointer);

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
