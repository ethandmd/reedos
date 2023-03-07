//! Kernel memory utilities
use core::ops::{Deref, DerefMut};
use core::mem::size_of;
use crate::vm::GALLOC;

/// Kernel heap allocated pointer. No guarantees on unique ownership
/// or concurrent access.
pub struct Kbox<T: ?Sized> {
    inner: *mut T, // NonNull<T>, try NonNull later for lazy allocation impl.
    size: usize,
}

impl<T> Kbox<T> {
    pub fn new(data: T) -> Self {
        // How the allocater interface should be made use of.
        // Current constraints on allocator mean size_of::<T>() must be less than 4Kb
        let size = size_of::<T>();
        match unsafe { (*GALLOC).alloc(size) } {
            Err(e) => {
                panic!("Kbox can't allocate: {:?}", e)
            },
            Ok(ptr) => {
                let new_ptr = ptr as *mut T;
                unsafe {
                    *new_ptr = data; // <-- initialize newly allocated memory with our inner value.
                    Self {
                        inner: new_ptr,
                        size
                    }
                }
            }
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
        unsafe {
            (*GALLOC).dealloc(self.inner as *mut usize, self.size);
        }
    }
}
