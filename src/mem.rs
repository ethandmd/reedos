//! Kernel memory utilities
use core::ops::{Deref, DerefMut};

/// Kernel heap allocated pointer. No guarantees on unique ownership or concurrent access.
pub struct Kbox<T: ?Sized> {
    inner: *mut T, // NonNull<T>, try NonNull later for lazy allocation impl.
}

impl<T> Kbox<T> {
    pub fn new(mut data: T) -> Self {
        // How the allocater interface should be made use of.
        // Current constraints on allocator mean size_of::<T>() must be less than 4Kb
        // let new_ptr = global allocator, allocate us size_of::<T>() bytes, please.
        // new_ptr.write(data); <-- initialize newly allocated memory with our inner value.
        //let new_ptr: *mut T = core::ptr::null_mut(); // Delete placeholder code.
        let inner = &mut data;
        Self {
            inner,
        }
    }
}

unsafe impl<T: ?Sized + Send + Sync> Send for Kbox<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for Kbox<T> {}

impl<T> Deref for Kbox<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.inner
        }
    }
}

impl<T> DerefMut for Kbox<T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.inner
        }
    }
}

impl<T: ?Sized> Drop for Kbox<T> {
    fn drop(&mut self) {
        // How to use the allocator interface.
        // dealloc(self.inner)
    }
}
