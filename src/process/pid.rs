/// This module is for abstracting PID generation and release, as it
/// is roughly as complex as any variable lifetime problem, including
/// allocation. For now I think we will be doing nothing particularly
/// special, at least until it starts to bother me, or we start to get
/// errors because of it.
///
/// Currently this is roughly a stack allocator.

use core::sync::atomic::{AtomicUsize, Ordering};


static mut PID_COUNTER: AtomicUsize = AtomicUsize::new(2);
// starts at 2 to save reserved values 0 and 1

/// Global inialization run once for the pid subsystem. Called from
/// process structure inialization.
pub fn init_pid_subsystem() {
    // just in case
    unsafe {
        assert!(PID_COUNTER.load(Ordering::Acquire) == 2);
    }
}

/// Get a new PID that is not currently in use.
pub fn generate_new_pid() -> usize {
    unsafe {
        let recieved = PID_COUNTER.fetch_add(1, Ordering::Acquire);
        if recieved == usize::MAX - 2 {
            panic!("Ran out of PIDs! Make a better PID system.");
        } else {
            recieved
        }
    }
}

/// Release a PID so that it can be used in the future.
pub fn return_used_pid(pid: usize) {
    unsafe {
        match PID_COUNTER.compare_exchange(
            pid,
            pid - 1,
            Ordering::Acquire,
            Ordering::Acquire   // TODO are these what I want?
        ) {
            Ok(_) => {
                // we were the last pid to get allocated, so we just
                // bumped it down
            },
            Err(_) => {
                // we were *not* the last pid to be allocated, so we
                // give up and our pid will just not be used again
            }
        }
    }
}
