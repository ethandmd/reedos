//! Spinlock RW lock implimentation
/// (Heavy) Inspiration taken from Mara Bos 'Rust Atomics and Locks'
/// book.
///
/// If there is an option to use a blocking lock of some kind, use
/// that. This is not something you want to be spinning on unless you
/// have to.
use core::sync::atomic::*;
use core::sync::atomic::Ordering::*;
use core::ops::{Deref, DerefMut};
use core::cell::UnsafeCell;
use core::hint::spin_loop;

pub struct RwLock<T> {
    // 2* the number of readers plus 1 if there is a waiting writer
    // u23::MAX if write locked
    state: AtomicU32,
    value: UnsafeCell<T>
}

// Must allow for concurrent reads w/o being sent, thus requires Sync
unsafe impl<T> Sync for RwLock<T> where T: Send + Sync {}


impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0), // unlocked to start,
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        let mut s = self.state.load(Relaxed);
        loop {
            if s % 2 == 0 {
                // reader held or unheld
                assert!(s != u32::MAX - 2, "Too many readers!");
                match self.state.compare_exchange_weak(
                    s, s + 2, Acquire, Relaxed
                ) {
                    Ok(_) => return ReadGuard { lock: self },
                    Err(e) => s = e,
                }
            } else {
                spin_loop();
                // writer held or writer blocked, block after them
                s = self.state.load(Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        let mut s = self.state.load(Relaxed);
        loop {
            // attempt lock
            if s <= 1 {
                // no readers, no writers
                // no spurious failures here
                match self.state.compare_exchange(
                    s, u32::MAX, Acquire, Relaxed
                ) {
                    Ok(_) => return WriteGuard { lock: self },
                    Err(e) => { s = e; continue; }
                }
            }
            // prevent new readers from starving this writer
            if s % 2 == 0 {
                match self.state.compare_exchange(
                    s, s + 1, Relaxed, Relaxed
                ) {
                    Ok(_) => {},
                    Err(e) => { s = e; continue; }
                }
            }
            // possibly wait if it is still locked
            spin_loop();
            s = self.state.load(Relaxed);
        }
    }
}

// differ only by traits and acquiring methods
pub struct WriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            &*self.lock.value.get()
        }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            &mut *self.lock.value.get()
        }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop (&mut self) {
        self.lock.state.store(0, Release);
        // would wake everyone if this was a blocking call
    }
}

pub struct ReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            &*self.lock.value.get()
        }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        if self.lock.state.fetch_sub(2, Release) == 3 {
            // last reader and writer is waiting. Do wakeups if this
            // was blocking lock
        }
    }
}
