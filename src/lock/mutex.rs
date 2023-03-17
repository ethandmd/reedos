//! Spinlock mutex implementation
/// Inspiration taken in no small part from the awesome:
/// <https://marabos.nl/atomics/building-locks.html#mutex>
///
/// Opportunity for improvement on interrupt safe locks.
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::*;

/// Returned from successfully locking a mutex.
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

/// A great Rust thing. Locking a mutex returns
/// a guard which derefs to the type behind the
/// mutex, which unlocks when it goes out of scope.
impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.inner.get() }
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.inner.get() }
    }
}

impl<T> core::ops::Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.lock_state.store(0, Ordering::Release);
    }
}

/// Simple mutex implementation.
/// 1. Try to acquire mutex for critical section.
/// 2. If unable, spin.
pub struct Mutex<T> {
    lock_state: AtomicU32, // (0,1) = (unlocked, locked)
    inner: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Reference:
    /// <https://doc.rust-lang.org/reference/const_eval.html>
    pub const fn new(value: T) -> Self {
        Mutex {
            lock_state: AtomicU32::new(0),
            inner: UnsafeCell::new(value),
        }
    }

    /// Needs to satisfy an atomic swap (acquire)
    /// then a fence so loads and stores aren't reordered until
    /// after lock is acquired.
    pub fn lock(&self) -> MutexGuard<T> {
        // Use Acquire memory order to load lock value.
        while self.lock_state.swap(1, Ordering::Acquire) == 1 {
            spin_loop();
        }
        MutexGuard { mutex: self }
    }
}
