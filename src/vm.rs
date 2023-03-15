//! Virtual Memory
pub mod palloc;
pub mod process;
pub mod ptable;
pub mod vmalloc;

use crate::hw::param::*;
use crate::mem::Kbox;
use core::cell::OnceCell;
use palloc::*;
use process::Process;
use ptable::kpage_init; //, PageTable};
use vmalloc::Kalloc;

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
static mut VMALLOC: OnceCell<Kalloc> = OnceCell::new();

/// (Still growing) list of kernel VM system error cases.
#[derive(Debug)]
pub enum VmError {
    OutOfPages,
    PartialPalloc,
    PallocFail,
    PfreeFail,
    GNoSpace,
    Koom,
}

/// Moving to `mod process`
pub trait Resource {}

/// Moving to `mod <TBD>`
pub struct TaskList {
    head: Option<Kbox<Process>>,
}

/// Moving to `mod <TBD>`
pub struct TaskNode {
    proc: Option<Kbox<Process>>,
    prev: Option<Kbox<TaskNode>>,
    next: Option<Kbox<TaskNode>>,
}

/// See `vm::vmalloc::Kalloc::alloc`.
pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
    unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
}

/// See `vm::vmalloc::Kalloc::free`.
pub fn kfree<T>(ptr: *mut T) {
    unsafe { VMALLOC.get_mut().unwrap().free(ptr) }
}

fn palloc() -> Result<Page, VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().palloc() }
}

fn pfree(page: Page) -> Result<(), VmError> {
    unsafe { PAGEPOOL.get_mut().unwrap().pfree(page) }
}

/// Initialize the kernel VM system.
/// First, setup the kernel physical page pool.
/// We start the pool at the end of the .bss section, and stop at the end of physical memory.
/// Next, we map physical memory into the kernel's physical memory 1:1.
/// Next, initialize the kernel virtual memory allocator pool.
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
pub fn init() -> Result<(), PagePool> {
    unsafe {
        match PAGEPOOL.set(PagePool::new(bss_end(), dram_end())) {
            Ok(_) => {}
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    unsafe {
        match palloc() {
            Ok(page) => {
                if VMALLOC.set(vmalloc::Kalloc::new(page)).is_err() {
                    panic!("VMALLOC double init...")
                }
            }
            Err(_) => panic!("Unable to allocate initial zone for vmalloc..."),
        }
    }

    // Map text, data, stacks, heap into kernel page table.
    match kpage_init() {
        Ok(pt) => pt.write_satp(),
        Err(_) => {
            panic!();
        }
    }
    Ok(())
}

/// A test designed to be used with GDB.
pub unsafe fn test_palloc() {
    let allocd = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    //println!("allocd addr: {:?}", allocd.addr);
    allocd.addr.write(0xdeadbeaf);
    let _ = PAGEPOOL.get_mut().unwrap().pfree(allocd);
    log!(Debug, "Successful test of page allocation and freeing...");
}

/// A test that is more insightful when run with GDB.
/// Likely missing some edge cases like:
///  + Free the last two chunks in a zone. Ensure you don't
///  try to merge out of zone bounds.
pub unsafe fn test_kalloc() {
    use core::mem::size_of;
    use core::ptr::write;
    struct Atest {
        xs: [u64; 4],
    }
    impl Atest {
        fn new() -> Self {
            let xs = [5; 4];
            Atest { xs }
        }
    }
    let addr1 = kalloc(8).expect("Could not allocate addr1...");
    assert_eq!(addr1.sub(2).read(), 0x1); // Check zone refs
    assert_eq!(addr1.sub(1).read(), 0x1008); // Check chunk header size + used
    addr1.write(0xdeadbeaf);

    let addr2: *mut [u64; 2] = kalloc(16).expect("Could not allocate addr3...").cast();
    assert_eq!(addr1.sub(2).read(), 0x2); // Check zone refs
    assert_eq!((addr2 as *mut usize).sub(1).read(), 0x1010); // Check chunk header size + used
    write(addr2, [0x8BADF00D, 0xBAADF00D]);

    let t = Atest::new();
    let addr3: *mut Atest = kalloc(size_of::<Atest>())
        .expect("Could not allocate addr3...")
        .cast();
    write(addr3, t);

    kfree(addr1);
    kfree(addr2);
    kfree(addr3);
    assert_eq!(addr1.sub(2).read(), 0x0); // Check zone refs
    assert_eq!((addr2 as *mut usize).sub(1).read(), 0x10); // Check chunk header size + used

    let addr4 = kalloc(0xfc0).expect("Could not allocate addr4...");
    let addr5 = kalloc(8).expect("Could not allocate addr5...");
    write(addr5, 0xee1f00d);
    kfree(addr5);
    kfree(addr4);

    let addr6: *mut [u64; 510] = kalloc(0xff0)
        .expect("Could not allocate addr6 (remainder of page)...")
        .cast();
    // Don't do this: Will stack overflow.
    // Foreboding for Kbox::new() correctness.
    // let big_xs = [555; 510];
    // unsafe { write(addr6, big_xs); }

    let addr7 = kalloc(9).expect("Could not allocate addr7...");
    kfree(addr6);
    kfree(addr7);

    log!(Debug, "Successful test of kalloc and kfree...");
}
