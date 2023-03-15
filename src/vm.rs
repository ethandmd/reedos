//! Virtual Memory
pub mod palloc;
pub mod ptable;
pub mod process;
pub mod galloc;
pub mod vmalloc;

use crate::hw::param::*;
use crate::mem::Kbox;
use palloc::*;
use ptable::kpage_init; //, PageTable};
use process::Process;
use core::cell::OnceCell;

/// Global physical page pool allocated by the kernel physical allocator.
static mut PAGEPOOL: OnceCell<PagePool> = OnceCell::new();
static mut VMALLOC: OnceCell<vmalloc::Kalloc> = OnceCell::new();

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

pub trait Resource {}

pub struct TaskList {
    head: Option<Kbox<Process>>,
}

pub struct TaskNode {
    proc: Option<Kbox<Process>>,
    prev: Option<Kbox<TaskNode>>,
    next: Option<Kbox<TaskNode>>,
}

pub fn kalloc(size: usize) -> Result<*mut usize, vmalloc::KallocError> {
    unsafe { VMALLOC.get_mut().unwrap().alloc(size) }
}

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
/// Finally we set the global kernel page table `KPGTABLE` variable to point to the
/// kernel's page table struct.
pub fn init() -> Result<(), PagePool>{
    unsafe {
        match PAGEPOOL.set(PagePool::new(bss_end(), dram_end())) {
            Ok(_) => {},
            Err(_) => {
                panic!("vm double init.")
            }
        }
    }
    log!(Debug, "Successfully initialized kernel page pool...");

    unsafe {
        match palloc() {
            Ok(page) => {
                if let Err(_) = VMALLOC.set(vmalloc::Kalloc::new(page)) {
                    panic!("VMALLOC double init...")
                }
            },
            Err(_) => panic!("Unable to allocate initial zone for vmalloc...")
        }
    }

    // Map text, data, stacks, heap into kernel page table.
    match kpage_init() {
        Ok(pt) => {
            pt.write_satp()
        },
        Err(_) => {
            panic!();
        }
    }
    Ok(())
}



pub unsafe fn test_palloc() {
    let allocd = PAGEPOOL.get_mut().unwrap().palloc().unwrap();
    //println!("allocd addr: {:?}", allocd.addr);
    allocd.addr.write(0xdeadbeaf);
    let _ = PAGEPOOL.get_mut().unwrap().pfree(allocd);
    log!(Debug, "Successful test of page allocation and freeing...");
}

pub fn test_kalloc() {
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
    unsafe { addr1.write(0xdeadbeaf); }
    
    let addr2: *mut [u64; 2] = kalloc(16).expect("Could not allocate addr3...").cast();
    unsafe { write(addr2, [0x8BADF00D, 0xBAADF00D]) };
    
    let t = Atest::new();
    let addr3: *mut Atest = kalloc(size_of::<Atest>()).expect("Could not allocate addr3...").cast();
    unsafe { write(addr3, t); }
    
    kfree(addr1);
    kfree(addr2);
    kfree(addr3);

    let addr4 = kalloc(0xfc0).expect("Could not allocate addr4...");
    kfree(addr4);
}






