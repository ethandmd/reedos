/// This module contains all the related stuff for blocking operations
/// or actions on blocking resources (a subset of all actions on
/// shared resources).
///
/// TODO get someone who knows something check all these lifetimes

use alloc::{collections::LinkedList, boxed::Box};

use crate::lock::mutex::Mutex;

// TODO I would love for this to be more generic, but I don't see any
// way
pub enum ReqType {
    Read,
    Write,
}

// This is so deeply not the point of rust, but I don't see a better
// way to do it. Even refernces require a metric ton of lifetime
// nonsense that is not viable here (trust me I tried)
pub union Reference<T: BlockAccess + ?Sized> {
    pub c: *const T,
    pub r: *mut T,
}

// TODO consider enforcing safety by making type a generic parameter
// instead of a member, and then implementing Deref[Mut] as
// appropriate? This would lead to even more dyn trait nonsense so I
// don't think that actually gets us any more safety but idk
pub struct ReqId<T: BlockAccess + ?Sized> {
    pub id: usize,
    pub req: ReqType,               // informs which reference to take
    pub contents: Reference<T>
}

pub enum ReqErr {
    InUse,
}

/// What should we do with the resource now that we have acquired it?
/// These functions (likely a closure), *MUST* return, and for safety
/// reasons should not interact with their or any other blocking
/// resource. Note also that this call is responsible for storing the
/// moved ReqId to a known / saved location (likely in the process
/// struct) so that it can be returned later.
pub type BlockContinuation<T> = Box<dyn FnOnce(ReqId<T>)>;

/// Allows a block to pass management decisions down to the underlying
/// resource.
pub trait Blockable<T: BlockAccess + ?Sized> {
    fn request(&mut self, req: &ReqType) -> Result<ReqId<T>, ReqErr>;
    fn release(&mut self, req: ReqId<T>);
}

/// TODO better name. This is an empty trait to allow generics for
/// Block over anything that a block can return, which it doesn't make
/// sense to be staticly generic over. This exists solely to be used
/// as dyn [this trait] in the process struct.
pub trait BlockAccess {}

/// Wrap this around a resource to allow for resource defined access
/// patterns. If the access is available, then execution resumes via
/// the continuation. If not, the continuation will be executed at a
/// later time, when the access is granted.
pub struct Block<T, R>
where T: BlockAccess + ?Sized,
      R: Blockable<T> + ?Sized  // allow for dyn
{
    inner: Mutex<(
        Box<R>,
        LinkedList<(
            ReqType,
            BlockContinuation<T>
        )>
    )>,
    // ugly but prevents deadlock opportunities
}

impl<R, T> Block<T, R>
where T: BlockAccess + ?Sized,
R: Blockable<T> + ?Sized
{
    /// Check if there are things in the blocked list that could be
    /// unblocked, and do so.
    fn try_drain(&self) {
        let mut lock = self.inner.lock();
        let mut req_queue: LinkedList<ReqId<T>> = LinkedList::new();
        let to_run: LinkedList<(ReqType, BlockContinuation<T>)>;
        unsafe {
            let res = (&mut lock.0) as *mut Box<R>;
            let list = (&mut lock.1) as *mut LinkedList<_>;
            to_run =
                (*list).drain_filter(
                    |(req, _cont)| match (*res).request(req) {
                        Ok(id) => {
                            // we should run this
                            req_queue.push_back(id);
                            return true
                        },
                        Err(_) => return false,
                    }).collect();
            // now to_run are things we can knock out, and the things left
            // in the locked list are still blocked
        }
        drop(lock);
        for (_req, cont) in to_run.into_iter() {
            cont(req_queue.pop_front().unwrap());
        }
    }

    /// Make a request to the underlying resource. Regardless of
    /// whether or not it can be fulfilled immediately, this function
    /// will always return in finite time. See the documentaiton of
    /// the continuation type for details about responsibilities, but
    /// in short the caller should ensure at a minimum that once the
    /// request is fulfilled, then progress can be made somewhere
    /// else.
    ///
    /// The clear example is that once the request is fulfilled, then
    /// you should change the process state from blocked to ready, so
    /// that on the next scheduling pass there is something to do,
    /// eventually leading to the release of the resource if there are
    /// other processes blocked here.
    pub fn request(&self, req: ReqType, cont: BlockContinuation<T>) {
        let mut lock = self.inner.lock();
        match lock.0.request(&req) {
            Ok(id) => {
                // we got it! get to work
                drop(lock);
                // ^ don't hold the lock into the continuation
                cont(id);
            },
            Err(e) => {
                // no dice, add to the blocked pile
                //
                // TODO might be more efficeient to spin here a bit
                lock.1.push_back((req, cont));
                drop(lock)
                // ^ now we will be woken when we can try again
            }
        }
        self.try_drain();
        // ^ Try to open as many possible sources of progress as
        // possible. Note that this may be redudant on some requests,
        // such as the one we just tried, but it simplies the program
        // flow here, so it will work for now
    }

    /// Inform the underlying resource that the access that your
    /// requested is no longer necessary. This should propogate back
    /// to anyone else that might be blocked here. It is the joint
    /// responsibility of the resource and the caller to ensure that
    /// everything is cleaned up. The caller should do cleanup before
    /// this call, and the resource should do cleanup inside the trait
    /// release call.
    pub fn release(&self, req: ReqId<T>) {
        let mut lock = self.inner.lock();
        lock.0.release(req);
        drop(lock);
        self.try_drain();
    }
}


