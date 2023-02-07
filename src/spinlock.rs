/// Inspiration taken in no small part from the awesome:
/// https://marabos.nl/atomics/building-locks.html#mutex
/// as well as:
/// https://github.com/westerndigitalcorporation/RISC-V-Linux/blob/master/linux/Documentation/locking/mutex-design.txt
use core::cell::UnsafeCell;
use core::sync::atomic::*;

// Use unit struct CriticalSection as a lifetime
// reference for holding on to a mutex.
pub struct CriticalSection;

impl CriticalSection {
    pub fn new() -> Self {
        CriticalSection
    }
}

// Simple mutex implementation.
// 1. Try to acquire mutex for critical section.
// 2. If unable, spin for a bit.
// 3. If still unable, sleep.
// 4. -> 1.
//
// We'll use similar Mutex -> MutexGuard construction,
// but instead of a guard with Deref and Drop traits,
// we'll have lock() directly return a ptr to T with inner.get().
// I know, living on the edge. BUT, we use the unit struct 
// critical section as a lifetime for T, thus it is safe...right?
pub struct Mutex<T> {
    lock_state: AtomicU32, // (0,1) = (unlocked, locked)
    inner: UnsafeCell<T>, // Not using SyncUnsafeCell bc don't feel like using nightly.
}

unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    // https://doc.rust-lang.org/reference/const_eval.html
    pub const fn new(value: T) -> Self {
        Mutex {
            lock_state: AtomicU32::new(0),
            inner: UnsafeCell::new(value),
        }
    }

    // Still debating just doing this in asm.
    pub fn lock<'crit>(&self, _crit: &'crit CriticalSection) -> &'crit T {
        // Use Acquire memory order to load lock value.
        // Yes, this is an indefinite spin. 
        // TODO:
        // Do not indefinitely spin.
        while self.lock_state.swap(1, Ordering::Acquire) == 1 {}
        unsafe { &*self.inner.get() }
    }
}


