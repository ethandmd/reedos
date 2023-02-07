/// Inspiration taken in no small part from the awesome:
///     https://marabos.nl/atomics/building-locks.html#mutex
///     https://github.com/westerndigitalcorporation/RISC-V-Linux/blob/master/linux/Documentation/locking/mutex-design.txt

use core::sync::atomic::AtomicU32;

pub struct Mutex<T> {
    locked: AtomicU32,
    inner: T
}
