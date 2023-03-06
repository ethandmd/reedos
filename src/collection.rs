use crate::mem::Kbox;

pub struct BalBst<T> {
    left: Option<Kbox<T>>,
    right: Option<Kbox<T>>,
}
