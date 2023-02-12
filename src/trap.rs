pub mod clint;

pub type __HANDLER = unsafe extern "C" fn();
pub static __TIMERVEC: unsafe extern "C" fn() = clint::timervec;
