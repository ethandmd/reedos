//! Kernel memory utilities

/// Kernel heap allocated pointer
pub struct Kbox<T: ?Sized> {
    inner: *mut T, // NonNull<T>, try NonNull later for lazy allocation impl.
}

impl<T> Kbox<T> {
    pub fn new(inner: T) -> Self {
        // How the allocater interface should be made use of.
        // let new_ptr = global allocator, allocate us size_of::<T>() bytes, please.
        // new_ptr.write(inner); <-- initialize newly allocated memory with our inner value.
        let new_ptr: *mut T = core::ptr::null_mut(); // Delete placeholder code.
        Self {
            inner: new_ptr as *mut T,
        }
    }
}
