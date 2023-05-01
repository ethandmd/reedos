//! Process handle and utilities.
// use alloc::boxed::Box;

// extern crate alloc;

use alloc::collections::{vec_deque::*, BTreeMap};
use alloc::rc::Rc;
// TODO ^ is this the right thing to use for blocking things?
use core::assert;
use core::mem::{size_of, MaybeUninit};
use core::ptr::{copy_nonoverlapping, null_mut};
use core::cell::OnceCell;

use crate::vm::ptable::*;
use crate::vm::VmError;
use crate::hw::riscv::read_tp;
use crate::hw::param::*;
use crate::vm::{request_phys_page, PhysPageExtent};
use crate::file::elf64::*;
use crate::hw::hartlocal::*;
use crate::lock::mutex::Mutex;
use crate::id::IdGenerator;


pub mod blocking;
use blocking::*;
// This should be visible externally

pub mod initialization;

mod scheduler;
use scheduler::ProcessQueue;

#[allow(unused_variables)]
mod syscall;
// This should not be exposed to anything, and we don't need to call
// any of it here

static mut PID_GENERATOR: OnceCell<Mutex<IdGenerator>> = OnceCell::new();

// for now we will be using a single locked round robin queue
static mut QUEUE: OnceCell<Mutex<ProcessQueue>> = OnceCell::new();

/// Global init for all process related stuff. Not exaustive, also
/// need hartlocal_info_interrupt_stack_init
pub fn init_process_structure() {
    unsafe {
        match PID_GENERATOR.set(Mutex::new(IdGenerator::new())) {
            Ok(()) => {},
            Err(_) => {
                panic!("Process structure double init!");
            },
        }
        match QUEUE.set(Mutex::new(ProcessQueue::new())) {
            Ok(()) => {},
            Err(_) => {
                panic!("Process structure double init!");
            },
        }
    }
}

/// Get a new PID that is not currently in use.
pub fn generate_new_pid() -> usize {
    unsafe {
        match PID_GENERATOR.get() {
            Some(lock) => {
                let mut guard = lock.lock();
                return guard.generate()
            },
            None => {
                panic!("Process structure not initalized!")
            },
        }
    }
}

/// Release a PID so that it can be used in the future.
pub fn return_used_pid(pid: usize) {
    unsafe {
        match PID_GENERATOR.get() {
            Some(lock) => {
                let mut guard = lock.lock();
                return guard.free(pid)
            },
            None => {
                panic!("Process structure not initalized!")
            },
        }
    }
}

// use hart local info to get the currently running process
//
// this is a *MOVE* of the process. Handle elsewhere
fn get_running_process() -> Process {
    restore_gp_info64().current_process
}

pub enum ProcessState<T>
where T: BlockAccess + ?Sized
{
    Uninitialized,              // do not attempt to run
    Unstarted,                  // do not attempt to restore regs
    Ready,                      // can run, restore args
    Running,                    // is running, don't use elsewhere
    // ^ is because ownership alone is risky to ensure safety accross
    // context switches
    Blocked(Rc<Block<T, dyn Blockable<T>>>),       // blocked on on something
    Sleep(u64),               // out of the running for a bit
    Dead,                       // do not run (needed?)
}

struct Resource<T>
where T: BlockAccess + ?Sized
{
    pair: (
        ReqId<T>,
        Rc<Block<T, dyn Blockable<T>>>
    )
}

struct ResourceList
{
    contents: BTreeMap<usize, Resource<dyn BlockAccess>>,
}

/// A process. The there is a real possiblity of this being largly
/// uninitialized, so check the state always
pub struct Process {
    saved_pc: usize,            // uninit with 0
    saved_sp: usize,            // uninit with 0
    id: usize,                  // uninit with 0
    state: ProcessState<dyn BlockAccess>,        // use uninit state
    pgtbl: PageTable,                     // uninizalied with null
    phys_pages: MaybeUninit<VecDeque<PhysPageExtent>>, // vec to avoid Ord requirement
    // ^ hopefully it's clear how this is uninit
    // TODO consider this as a OnceCell
    resource_id_gen: IdGenerator,
    held_resources: MaybeUninit<ResourceList>
    // ^ What are we holding and who do we return it to

    // currently unused, but needed in the future
    // address_space: BTreeSet<Box<dyn Resource>>, // todo: Balanced BST of Resources
}

impl Process {
    /// This is a (kind of) context switch
    ///
    /// This consumes the process from the rust perspective, but it is
    /// actually preserved elsewhere (gp info) and restored. This is
    /// because we need to preserve info across entering and exiting
    /// the process, but no non-global rust location does that, and we
    /// can't use a global array or anything like htat because we need
    /// to have each hart's process's lifetime be independent, and
    /// further, it doesn't make sense to have Process be Sync when it
    /// is not.
    ///
    /// TODO consider if there is a non-gp solution involing global
    /// pointers to heap allocated locations per hart. That is
    /// conceptually what is going on, but I still think we would have
    /// Sync/Send issues
    pub fn start(mut self) -> ! {
        match self.state {
            ProcessState::Unstarted => {},
            _ => {panic!("Attempted to start an already started program!")},
        }
        self.state = ProcessState::Running;

        extern "C" {pub fn process_start_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}

        let saved_pc = self.saved_pc;
        let pgtbl_base = self.pgtbl.base as usize;
        let saved_sp = self.saved_sp;
        let gpi = GPInfo::new(self);
        save_gp_info64(gpi);

        unsafe {
            // we can't use PageTable.write_satp here becuase this is
            // not mapped into the process pagetable and it shouldn't
            // be. We want to do that later in the asm.
            //
            // relies on args in a0, a1, a2 in order
            process_start_asm(saved_pc, pgtbl_base, saved_sp);
        }
    }

    /// This is our main context switch. Back into a running process
    /// from kernel space
    ///
    /// See above comment about data movement of a process struct
    pub fn resume(mut self) -> ! {
        match self.state {
            ProcessState::Ready => {},
            _ => {
                panic!("Attempted to resume a process that was not marked as Ready.")
            },
        }
        self.state = ProcessState::Running;

        extern "C" {pub fn process_resume_asm(pc: usize, pgtbl: usize, sp: usize) -> !;}

        let saved_pc = self.saved_pc;
        let pgtbl_base = self.pgtbl.base as usize;
        let saved_sp = self.saved_sp;
        let gpi = GPInfo::new(self);
        save_gp_info64(gpi);

        unsafe {
            process_resume_asm(saved_pc, pgtbl_base, saved_sp);
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        match self.state {
            ProcessState::Running => {
                panic!("Tried to drop a running process!");
            }
            _ => {}
        }
        return_used_pid(self.id);
        // dropping the phys pages vector will automatically clean
        // those up
    }
}

/// Suspend process so that it can be restored/restarted later. Called
/// from syscalls currently
fn process_pause(pc: usize, sp: usize, cause: usize) -> ! {
    let mut proc = get_running_process();
    proc.saved_pc = pc + 4;
    // ^ ecall doesn't automatically increment pc
    proc.saved_sp = sp;
    // TODO enum for causes?
    match cause {
        0 => {
            proc.state = ProcessState::Ready;
        },
        _ => {
            panic!("Unknown reason for process swap.");
        }
    }

    log!(Debug, "Hart {}: Process {} yielded.", read_tp(), proc.id);


    // This is careful code to avoid holding the lock when we enter
    // the process, as that would lead to an infinite lock
    let next;
    unsafe {
        let mut locked = QUEUE.get().unwrap().lock();
        locked.insert(proc);
        next = locked.get_ready_process();
    }
    match next.state {
        ProcessState::Ready => {next.resume()},
        ProcessState::Unstarted => {next.start()},
        _ => {panic!("Bad process state from scheduler!")}
    }
}

#[no_mangle]
pub extern "C" fn process_exit_rust(exit_code: isize) -> ! {
    let proc = get_running_process();
    log!(Debug, "Process {} exited with code {}.", proc.id, exit_code);
    drop(proc);
    // ^ ensure that the never returning scheduler call doesn't extend
    // the life of the process


    // This is careful code to avoid holding the lock when we enter
    // the process, as that would lead to an infinite lock
    let next;
    unsafe {
        let mut locked = QUEUE.get().unwrap().lock();
        next = locked.get_ready_process();
    }
    match next.state {
        ProcessState::Ready => {next.resume()},
        ProcessState::Unstarted => {next.start()},
        _ => {panic!("Bad process state from scheduler!")}
    }
}


pub fn _test_process_spin() {
    let bytes = include_bytes!("programs/spin/spin.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit();

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }
    proc.start();
}

pub fn _test_process_syscall_basic() {
    let bytes = include_bytes!("programs/syscall-basic/syscall-basic.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit();

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }
    proc.start();
}

pub fn test_multiprocess_syscall() {
    let bytes = include_bytes!("programs/syscall-basic/syscall-basic.elf");
    let program = ELFProgram::new64(&bytes[0] as *const u8);
    let mut proc = Process::new_uninit();

    match proc.initialize64(&program) {
        Ok(_) => {},
        Err(e) => {panic!("Couldn't start process: {:?}", e)}
    }

    for _ in 0..4 {
        let mut proc = Process::new_uninit();

        match proc.initialize64(&program) {
            Ok(_) => {},
            Err(e) => {panic!("Couldn't start process: {:?}", e)}
        }

        unsafe {
            QUEUE.get().unwrap().lock().insert(proc)
        }
    }

    let enter;
    unsafe {
        enter = QUEUE.get().unwrap().lock().get_ready_process();
    }
    match enter.state {
        ProcessState::Unstarted => enter.start(),
        ProcessState::Ready => enter.resume(),
        _ => {panic!()}
    }

}

