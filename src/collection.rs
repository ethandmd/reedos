use core::ptr::NonNull;

pub struct BalBst<T> {
    left: Option<NonNull<T>>,
    right: Option<NonNull<T>>,
}
