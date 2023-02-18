use crate::lock::mutex::Mutex;
use crate::alloc::Kalloc;
use crate::param::NHART;

use core::array::from_fn;

pub struct Kpools {
    global: [Mutex<Kalloc>; NHART+1], 
}

impl Kpools {
    // Set up locked local pools per hart
    // + locked global pool.
    pub fn new(start: usize, end: usize) -> Self{
        //let mut global: [Mutex<Kalloc>; NHART+1] = from_fn(|id| Mutex::new(Kalloc::new()));
        let local_size = (end-start) / 2*NHART;
        // Round down to power of 2
        let local_size = local_size >> 1;
        let local_size = local_size << 1;
 
        let global: [Mutex<Kalloc>; NHART+1] = from_fn(|id| {
            let local_start = start + local_size * id;
            let global_start = start + (local_size * (NHART + 1));
            if id <= NHART {
                return Mutex::new(Kalloc::new(local_start, local_start + local_size));
            } else {
                return Mutex::new(Kalloc::new(global_start, end));
            }});

        Kpools { global }
    }
}

// Address Space is :
// struct AS {
//     data: impl DataSource,
// }

// VA: 39bits, PA: 56bits
// struct Pte { pte: u64 }
// impl .v, .r, .x

// walk pagetable
// map PTE
// unmap PTE
// flush
