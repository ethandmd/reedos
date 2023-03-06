use core::ptr::NonNull;

pub struct Kbox<T: ?Sized>(NonNull<T>);
