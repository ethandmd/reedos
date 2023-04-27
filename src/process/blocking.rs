/// This module contains all the related stuff for blocking operations
/// or actions on blocking resources (a subset of all actions on
/// shared resources).

use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut, Drop};

type ReqType = u32;
type IdType = u32;

// General idea for the moment is that you call each of these on your
// implementing type, and while you hold a Blocking___Guard, you can
// safely perform the associated actions. Acquire methods should be
// used in user code, and release should only be used inside the drop
// of the associated guard.
//
// It is the responsibility of the implementor of this trait that they
// only issue a guard when it is safe for the associated actions to
// occur, and that they *remain* safe until they are released.
pub unsafe trait Blocking<'a, T> {
    fn acquire_read(&'a mut self) -> Option<BlockingReadGuard<'a, T>>;
    fn acquire_write(&'a mut self) -> Option<BlockingWriteGuard<'a, T>>;
    fn release_read(&mut self, guard: &mut BlockingReadGuard<T>);
    fn release_write(&mut self, guard: &mut BlockingWriteGuard<T>);
}

pub struct Block<'a, T> {
    resource: Box<UnsafeCell<dyn Blocking<'a, T>>>,
}

// -------------------------------------------------------------------

pub struct BlockingReadGuard<'a, T> {
    id: IdType,                 // for internal identification
    value: &'a T,               // for the actions
    block: &'a Block<'a, T>,    // used on release
}

impl<'a, T> Deref for BlockingReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> Drop for BlockingReadGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { (*self.block.resource.get()).release_read(self) }
    }
}

// -------------------------------------------------------------------

pub struct BlockingWriteGuard<'a, T> {
    id: IdType,                 // for internal identification
    value: &'a mut T,           // for the actions
    block: &'a Block<'a, T>,    // used for release
}

impl<'a, T> Deref for BlockingWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> DerefMut for BlockingWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<'a, T> Drop for BlockingWriteGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { (*self.block.resource.get()).release_write(self) }
    }
}

