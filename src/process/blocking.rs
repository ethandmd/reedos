/// This module contains all the related stuff for blocking operations
/// or actions on blocking resources (a subset of all actions on
/// shared resources).
///
/// TODO get someone who knows something check all these lifetimes

use core::ops::{Deref, DerefMut, Drop};

pub enum ReqType {
    Read,
    Write,
}

type IdType = u32;

pub trait Resource {}
// ^ Right now this just allows us to be generic at runtime over T,
// but later it might also allow to enforce other things

// General idea for the moment is that you call each of these on your
// implementing type, and while you hold a Blocking___Guard, you can
// safely perform the associated actions. Acquire methods should be
// used in user code, and release should only be used inside the drop
// of the associated guard.
//
// It is the responsibility of the implementor of this trait that they
// only issue a guard when it is safe for the associated actions to
// occur, and that they *remain* safe until they are released.
//
// The lifetime of the resource T cannot exceed the lifetime of the
// struct implementing Blocking<T>, and therefore none of the
// references in a guard can outlive the block either.

pub unsafe trait Blocking<T: Resource + ?Sized> {
    fn acquire_read(&self) -> Option<BlockingReadGuard<T>>;
    fn acquire_write(&self) -> Option<BlockingWriteGuard<T>>;
    fn release_read(&self, guard: &mut BlockingReadGuard<T>);
    fn release_write(&self, guard: &mut BlockingWriteGuard<T>);
}

// The lifetime of the block must not exceed the lifetime of the
// underlying Blocking Type. It can outlive the returned guards,
// however.
// pub struct Block {
//     resource: Box<UnsafeCell<dyn Blocking<dyn Resource>>>,
// }

// impl<'a> Block
// where Self: 'a,
// {
//     pub fn acquire_read<T: Resource>(&self) -> Option<BlockingReadGuard<'a, T>> {
//         unsafe { Blocking::acquire_read(&(*self.resource.get())) }
//     }

//     pub fn acquire_write(&self) -> Option<BlockingWriteGuard<'a, T>> {
//         unsafe { (*self.resource.get()).acquire_write() }
//     }
// }

// -------------------------------------------------------------------

// None of these references can outlive the guard
pub struct BlockingReadGuard<'a, T>
where Self: 'a,
      T: Resource + ?Sized
{
    id: IdType,                 // for internal identification
    value: &'a T,               // for the actions
    resource: &'a dyn Blocking<T>,    // used on release
}

impl<'a, T> Deref for BlockingReadGuard<'a, T>
where Self: 'a,
      T: Resource
{
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<T> Drop for BlockingReadGuard<'_, T>
where T: Resource + ?Sized
{
    fn drop(&mut self) {
        self.resource.release_read(self)
    }
}

// -------------------------------------------------------------------

// None of these references can outlive the guard
pub struct BlockingWriteGuard<'a, T>
where Self: 'a,
      T: Resource + ?Sized
{
    id: IdType,                 // for internal identification
    value: &'a mut T,           // for the actions
    resource: &'a dyn Blocking<T>,    // used on release
}

impl<'a, T> Deref for BlockingWriteGuard<'a, T>
where Self: 'a,
      T: Resource + ?Sized
{
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T> DerefMut for BlockingWriteGuard<'a, T>
where Self: 'a,
      T: Resource
{
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T> Drop for BlockingWriteGuard<'_, T>
where T: Resource + ?Sized
{
    fn drop(&mut self) {
        self.resource.release_write(self)
    }
}

