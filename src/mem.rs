//! Kernel memory utilities
use core::ops::{Deref, DerefMut};
use core::mem::size_of_val;
use crate::vm::{kalloc, kfree};

/// Kernel heap allocated pointer. No guarantees on unique ownership
/// or concurrent access.
pub struct Kbox<T: ?Sized> {
    inner: *mut T, // NonNull<T>, try NonNull later for lazy allocation impl.
    size: usize,
}

impl<T: ?Sized> Kbox<T> {
    /// Note that as this exists currently, data is passed by value
    /// into new, which means that the initial contents of a box MUST
    /// be composed on the stack and passed here to be copied into the
    /// heap. Kbox contents will not change size during their
    /// lifetime, so it must soak up as much stack space as it will
    /// ever use.
    ///
    /// Also this may entail a stack->stack copy into this callee's
    /// stack fram, I am not sure. It might be optimized as a pass by
    /// reference with the compiler knowledge that it is a move under
    /// the hood, but I really can't say.
    pub fn new(data: T) -> Self {
        // How the allocater interface should be made use of.
        // Current constraints on allocator mean size_of::<T>() must be less than 4Kb
        let size = size_of_val::<T>(&data);
        match kalloc(size)  {
            Err(e) => {
                panic!("Kbox can't allocate: {:?}", e)
            },
            Ok(ptr) => {
                let new_ptr: *mut T = ptr.cast();
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
        kfree(self.inner as *mut usize, self.size);
    }
}
