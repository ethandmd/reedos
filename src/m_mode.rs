
pub mod m_mode {
    use core::arch::asm;

    fn set_vec(vec: &fn ()) {
        let addr: isize = vec as isize;
        unsafe {
            asm!("csrw mvec, {ad}",
             ad = in(reg) addr);
        }
    }

    fn trap_get_cause() -> u64 {
        let mut output: u64 = 0;
        unsafe {
            asm!("csrrc {op}, mcause, x0",
             op = out(reg) output)
        }
        return output;
    }

    fn trap_vec () {
        let cause: u64 = trap_get_cause();

        println!("Machine mode trap fired: Cause {}", cause);

    }


    pub fn setup_trap() {
        set_vec(trap_vec);
    }
}
