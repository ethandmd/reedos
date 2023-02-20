pub mod palloc;
pub mod ptable;

extern "C" {
    pub static __bss_end: usize;
    pub static __memory_end: usize;
}
