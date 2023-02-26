use crate::lock::mutex::Mutex;
use crate::param::PAGE_SIZE;

enum KpoolError {
    FreeFail,
}

struct Freedom {
    curr: *mut usize,
    size : usize,
}

pub struct Kpools {
    start: *const usize,
    end: *const usize,
    curr: *mut usize,
    free: Option<Freedom>,
}

unsafe impl Sync for Kpools {}

impl Kpools {
    pub fn new(start: *const usize, end: *const usize) -> Mutex<Self> {
        Mutex::new(Kpools {
            start,
            end,
            curr: start as *mut usize,
            free: None
        })
    }

    pub fn palloc(&mut self, num_pages: usize) -> Option<*mut usize> {
        if self.curr as usize + (num_pages * PAGE_SIZE) as usize >= self.end as usize {
            return None;
        } else {
            let ret = self.curr;
            self.curr = unsafe { self.curr.byte_add(PAGE_SIZE*num_pages) } ;
            Some(ret)
        }
    }

    pub fn pfree() {
        panic!();
    }
}
