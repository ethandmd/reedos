/// This module isolates all the syscall stuff written in rust. See
/// syscall.s for the asm half of this

use core::arch::asm;
use alloc::slice;

use crate::device::uart;
use super::*;

/// This is called from asm proc_space_to_kernel_space, and handles
/// the actions that should happen in kernel space on behalf of a
/// process. We call that a "kernel excursion" and they *must* have
/// standard rust contiguous program flow, so no further jumping
/// about. Further they must call the asm kernel_space_to_proc_space
/// when they are done.
#[no_mangle]
pub extern "C" fn kernel_excursion_rust() -> ! {
    let proc_sp: usize;
    unsafe {
        asm!(
            "mv {sp}, s3",
            sp = out(reg) proc_sp
        );
    }
    let mut gpi = restore_gp_info64();
    gpi.current_process.saved_sp = proc_sp;
    match gpi.cause {
        GPCause::Read(fid, size) => {
            // size safety is checked in syscall handler

            if fid == 0 {
                todo!("UART process input");
            }

            // we need to copy from file backing into the proc/file
            // pair buffer.
            let file_buf;
            unsafe {
                let file_pair = gpi.current_process.file_buffers
                    .assume_init_ref().get(&fid)
                    .unwrap();

                file_buf = slice::from_raw_parts(file_pair.1.start() as *mut u8, size);
            }

            // now fill the buffer
            todo!("This is where I would call the fs layer read. Consider slice vs explicit length + addr pair.");

            // pass through info from fs layer
            gpi.ret = todo!("Either num bytes or errno associated as result");

            // back to the syscall handler
            let base_addr = gpi.current_process.pgtbl.base as usize;
            save_gp_info64(gpi);

            extern "C" { pub fn kernel_space_to_proc_space(sp: usize, pgtbl: usize) -> !; }
            unsafe {
                kernel_space_to_proc_space(proc_sp, base_addr);
            }

        },
        GPCause::Write(fid, size) => {

            let input_range;
            unsafe {
                let ppe = &gpi.current_process.file_buffers
                    .assume_init_ref().get(&fid)
                    .unwrap().1;  // handled earlier
                input_range = slice::from_raw_parts(ppe.start() as *mut u8, size)
            }

            if fid == 1 || fid == 2 {
                // hardcoded uart for std out/err
                let mut uart;
                unsafe {
                    uart = uart::WRITER.lock();
                }

                if fid == 2 {
                    print!("PROC {} ERR: ", gpi.current_process.id);
                }

                for c in input_range {
                    uart.put(c.clone());
                    // TODO can this fail?
                }

                drop(uart);
            } else {
                todo!("This is where I would do a fs layer write call from the input buffer");

                gpi.ret = todo!("pass along info from fs layer, ret val or errno");
            }

            let base_addr = gpi.current_process.pgtbl.base as usize;
            save_gp_info64(gpi);

            extern "C" { pub fn kernel_space_to_proc_space(sp: usize, pgtbl: usize) -> !; }
            unsafe {
                kernel_space_to_proc_space(proc_sp, base_addr);
            }
        },
        GPCause::None => panic!("Kernel excursion without cause"),
    }
}

/// System call rust handler. This is called from scall_asm. See there
/// for calling convention info.
///
/// This function is odd, because it runs EITHER in the kernel stack
/// with the kernel page table, or with the process stack and page
/// table. This choice is determined by calling `scall_direct` with
/// the same arguments. With that in mind, this function is
/// responsible for multiplexing into whatever call was actually
/// issued.
///
/// If you entered this function on the kernel stack/page table, when
/// you are done here, you should use a process resume instead of
/// returning from this function.
///
/// This may change in the future.
#[no_mangle]
pub extern "C" fn scall_rust(a0: usize, a1: usize, a2: usize, a3: usize,
                             a4: usize, a5: usize, a6: usize, a7: usize) -> isize {
    match a7 {
        SCHED_YIELD => {
            // see the comment on scall_direct for why we have these
            let proc_pc: usize;
            let proc_sp: usize;
            unsafe {
                asm!(
                    "mv {pc}, s2",
                    "mv {sp}, s3",
                    pc = out(reg) proc_pc,
                    sp = out(reg) proc_sp
                );
            }
            process_pause(proc_pc, proc_sp, 0); // cause 0, explicit yield
        },
        // -----------------------------------------------------------
        // File stuff
        READ | WRITE => {
            // this is a file operation, we are in the process space
            //
            // a0: file id
            // a1: in/out buffer addr in the process
            // a2: number of bytes to read

            // see the comment on scall_direct for why we have these
            let proc_pc: usize;
            unsafe {
                asm!(
                    "mv {pc}, s2",
                    pc = out(reg) proc_pc,
                );
            }

            let mut gpi = restore_gp_info64();
            let proc = &mut gpi.current_process;

            // proc.saved_sp = proc_sp; // not sure I need this
            proc.saved_pc = proc_pc + 4;

            let buffer_range;
            unsafe {
                let buf = &proc.file_buffers.assume_init_ref().get(&a0)
                    .expect("Read called on unknown fid").1;

                if buf.length() < a2 {
                    todo!("Oversized file operation from process!");
                }

                buffer_range = slice::from_raw_parts_mut(buf.start() as *mut u8, a2);
            }
            match a7 {
                READ => unsafe {
                    gpi.cause = GPCause::Read(a0,a2);
                    save_gp_info64(gpi);
                    extern "C" { pub fn proc_space_to_kernel_space(); }
                    proc_space_to_kernel_space();
                    // this is a kernel excursion, see
                    // trampoline.s and the function above
                    gpi = restore_gp_info64();

                    // the kernel has filled the given buffer
                    // associated with the process/file pair, now copy
                    // into the target buffer
                    let target_range = slice::from_raw_parts_mut(a1 as *mut u8, a2);
                    target_range.copy_from_slice(buffer_range);
                    match gpi.ret {
                        Ok(bytes) => bytes as isize,
                        Err(errno) => -1,
                    }
                },
                WRITE => unsafe {
                    let input_range = slice::from_raw_parts(a1 as *mut u8, a2);
                    buffer_range.copy_from_slice(input_range);
                    // the data is now in the kernel buffer for that file, we can move to kernel space
                    gpi.cause = GPCause::Write(a0, a2);
                    save_gp_info64(gpi);

                    extern "C" { pub fn proc_space_to_kernel_space(); }
                    proc_space_to_kernel_space();
                    // this is a kernel excursion, see
                    // trampoline.s and the function above
                    gpi = restore_gp_info64();
                    match gpi.ret {
                        Ok(bytes) => bytes as isize,
                        Err(errno) => -1,
                    }
                },
                _ => panic!("Wrote to a7 in syscall handler")
            }
        },
        OPENAT => {
            // a0 file desc (fid) of directory, or special AT_FDCWD (TODO), must be opened O_RDONLY or O_PATH
            // a1 *const u8 to null terminated string path
            // ^ if relative, relative to a0 dir, if absolute, a0 is ignored
            // a2 is a bitfield of the flags
            // a3 "mode" is listed in the man page? but not seemly used? Totally unclear
            let proc_pc: usize;
            let proc_sp: usize;
            unsafe {
                asm!(
                    "mv {pc}, s2",
                    "mv {sp}, s3",
                    pc = out(reg) proc_pc,
                    sp = out(reg) proc_sp
                );
            }

            let gpi = restore_gp_info64();
            let mut proc = gpi.current_process;

            proc.saved_sp = proc_sp;
            proc.saved_pc = proc_pc + 4;
            proc.state = ProcessState::Ready;

            unsafe {
                assert!(*(a1 as *const u8) == '/' as u8, "Only abosolute paths are supported at this time");
                assert!(a3 == 0, "Non zero mode on openat. What's the deal?");
            }
            let file: FileHandle = todo!("fs layer lookup with perms, catch errors for GP info here too");

            let id = proc.file_id_gen.generate();
            unsafe {
                let fb = proc.file_buffers.assume_init_mut();
                fb.insert(id, (Some(file), request_phys_page(1)
                              .expect("Could not allocate file buffer for process")));
            }
            save_gp_info64(gpi);
            proc.resume(Ok(id));
        },
        CLOSE => {
            // a0 is file id
            // should return 0 on success or appropriate errno with gpi scheme
            let proc_pc: usize;
            let proc_sp: usize;
            unsafe {
                asm!(
                    "mv {pc}, s2",
                    "mv {sp}, s3",
                    pc = out(reg) proc_pc,
                    sp = out(reg) proc_sp
                );
            }

            let gpi = restore_gp_info64();
            let mut proc = gpi.current_process;

            proc.saved_sp = proc_sp; // not sure I need this
            proc.saved_pc = proc_pc + 4;
            proc.state = ProcessState::Ready;


            let ret_val;
            unsafe {
                match proc.file_buffers.assume_init_mut().remove(&a0) {
                    Some((fh, ppe)) => {
                        match fh {
                            Some(file) => {
                                // we are actually closing a file properly
                                let io_err = todo!("fs layer close call on file");
                                // match on ^ and do errno stuff for io error

                                drop(ppe); // return pages
                                ret_val = Ok(0);
                            },
                            None => panic!("Tried to close a file without file backing. Did you close a pipe?"),
                        }
                    },
                    None => {
                        // wasn't an open fd
                        ret_val = Err(todo!("errno for bad file desc"));
                    },
                }
            }
            save_gp_info64(gpi);
            proc.resume(ret_val);
        },
        // -----------------------------------------------------------
        // Other
        _ => {
            panic!("Uncaught system call: {}", a7);
        }
    }
}

/// This call runs in the sscratch stack, and serves only to direct
/// traffic, deciding whether further scall processing happens on the
/// kernel stack with the kernel page table or the process stack with
/// the process page table.
///
/// This function returns 0 if the syscall should execute on the
/// program stack/page table, and any non-zero value to execute on the kernel
/// stack/page table.
///
/// The return value here is in a0. This function must also have a net
/// stack impact of zero. This function cannot overwrite its
/// arguments. These conditions are expected by the asm.
///
/// In addition, if this function returns non-zero, then this scall is
/// considered to have left the process, at least partially. This
/// means that as of entering `scall_rust`, the process registers have
/// been saved to the process stack, we are now in kernel space, the
/// process pc is in s2, and the process sp is in s3
#[no_mangle]
pub extern "C" fn scall_direct(a0: usize, a1: usize, a2: usize, a3: usize,
                               a4: usize, a5: usize, a6: usize, a7: usize)
                               -> usize {
    match a7 {
        SCHED_YIELD => 1,

        // File stuff
        READ => 0,
        WRITE => 0,
        OPENAT => 1,
        CLOSE => 1,

        // other
        _ => {
            0
        }
    }
}

/// Default handler for syscalls that aren't yet implemented


// -------------------------------------------------------------------
//
// Just a lot of constants down here.
//

// These are the RISC-V Linux syscall numbers
//
// I'd love for them to be an enum, but those aren't transparent over
// integers like in C, so it gets messy fast
//
// Info on these can be found with $man 2 {name}, but you are probably
// not running riscv
//
// TODO these are hardware specific, and thus it might be nice to
// abstract them elsewhere, but also there is literally no way to
// write this kind of stuff in a way that is not hardware
// specific. Also these should reasonably only be used here.

pub const IO_SETUP: usize = 0;
pub const IO_DESTROY: usize = 1;
pub const IO_SUBMIT: usize = 2;
pub const IO_CANCEL: usize = 3;
pub const IO_GETEVENTS: usize = 4;
pub const SETXATTR: usize = 5;
pub const LSETXATTR: usize = 6;
pub const FSETXATTR: usize = 7;
pub const GETXATTR: usize = 8;
pub const LGETXATTR: usize = 9;
pub const FGETXATTR: usize = 10;
pub const LISTXATTR: usize = 11;
pub const LLISTXATTR: usize = 12;
pub const FLISTXATTR: usize = 13;
pub const REMOVEXATTR: usize = 14;
pub const LREMOVEXATTR: usize = 15;
pub const FREMOVEXATTR: usize = 16;
pub const GETCWD: usize = 17;
pub const LOOKUP_DCOOKIE: usize = 18;
pub const EVENTFD2: usize = 19;
pub const EPOLL_CREATE1: usize = 20;
pub const EPOLL_CTL: usize = 21;
pub const EPOLL_PWAIT: usize = 22;
pub const DUP: usize = 23;
pub const DUP3: usize = 24;
pub const FCNTL64: usize = 25;
pub const INOTIFY_INIT1: usize = 26;
pub const INOTIFY_ADD_WATCH: usize = 27;
pub const INOTIFY_RM_WATCH: usize = 28;
pub const IOCTL: usize = 29;
pub const IOPRIO_SET: usize = 30;
pub const IOPRIO_GET: usize = 31;
pub const FLOCK: usize = 32;
pub const MKNODAT: usize = 33;
pub const MKDIRAT: usize = 34;
pub const UNLINKAT: usize = 35;
pub const SYMLINKAT: usize = 36;
pub const LINKAT: usize = 37;
pub const RENAMEAT: usize = 38;
pub const UMOUNT: usize = 39;
pub const MOUNT: usize = 40;
pub const PIVOT_ROOT: usize = 41;
pub const NI_SYSCALL: usize = 42;
pub const STATFS64: usize = 43;
pub const FSTATFS64: usize = 44;
pub const TRUNCATE64: usize = 45;
pub const FTRUNCATE64: usize = 46;
pub const FALLOCATE: usize = 47;
pub const FACCESSAT: usize = 48;
pub const CHDIR: usize = 49;
pub const FCHDIR: usize = 50;
pub const CHROOT: usize = 51;
pub const FCHMOD: usize = 52;
pub const FCHMODAT: usize = 53;
pub const FCHOWNAT: usize = 54;
pub const FCHOWN: usize = 55;
pub const OPENAT: usize = 56;
pub const CLOSE: usize = 57;
pub const VHANGUP: usize = 58;
pub const PIPE2: usize = 59;
pub const QUOTACTL: usize = 60;
pub const GETDENTS64: usize = 61;
pub const LSEEK: usize = 62;
pub const READ: usize = 63;
pub const WRITE: usize = 64;
pub const READV: usize = 65;
pub const WRITEV: usize = 66;
pub const PREAD64: usize = 67;
pub const PWRITE64: usize = 68;
pub const PREADV: usize = 69;
pub const PWRITEV: usize = 70;
pub const SENDFILE64: usize = 71;
pub const PSELECT6_TIME32: usize = 72;
pub const PPOLL_TIME32: usize = 73;
pub const SIGNALFD4: usize = 74;
pub const VMSPLICE: usize = 75;
pub const SPLICE: usize = 76;
pub const TEE: usize = 77;
pub const READLINKAT: usize = 78;
pub const NEWFSTATAT: usize = 79;
pub const NEWFSTAT: usize = 80;
pub const SYNC: usize = 81;
pub const FSYNC: usize = 82;
pub const FDATASYNC: usize = 83;
// pub const sync_file_range2: usize = 84,  // Not clear why this was included twice, leaving for posterity
pub const SYNC_FILE_RANGE: usize = 84;
pub const TIMERFD_CREATE: usize = 85;
pub const TIMERFD_SETTIME: usize = 411;
pub const TIMERFD_GETTIME: usize = 410;
pub const UTIMENSAT: usize = 412;
pub const ACCT: usize = 89;
pub const CAPGET: usize = 90;
pub const CAPSET: usize = 91;
pub const PERSONALITY: usize = 92;
pub const EXIT: usize = 93;
pub const EXIT_GROUP: usize = 94;
pub const WAITID: usize = 95;
pub const SET_TID_ADDRESS: usize = 96;
pub const UNSHARE: usize = 97;
pub const FUTEX: usize = 422;
pub const SET_ROBUST_LIST: usize = 99;
pub const GET_ROBUST_LIST: usize = 100;
pub const NANOSLEEP: usize = 101;
pub const GETITIMER: usize = 102;
pub const SETITIMER: usize = 103;
pub const KEXEC_LOAD: usize = 104;
pub const INIT_MODULE: usize = 105;
pub const DELETE_MODULE: usize = 106;
pub const TIMER_CREATE: usize = 107;
pub const TIMER_GETTIME: usize = 408;
pub const TIMER_GETOVERRUN: usize = 109;
pub const TIMER_SETTIME: usize = 409;
pub const TIMER_DELETE: usize = 111;
pub const CLOCK_SETTIME: usize = 404;
pub const CLOCK_GETTIME: usize = 403;
pub const CLOCK_GETRES: usize = 406;
pub const CLOCK_NANOSLEEP: usize = 407;
pub const SYSLOG: usize = 116;
pub const PTRACE: usize = 117;
pub const SCHED_SETPARAM: usize = 118;
pub const SCHED_SETSCHEDULER: usize = 119;
pub const SCHED_GETSCHEDULER: usize = 120;
pub const SCHED_GETPARAM: usize = 121;
pub const SCHED_SETAFFINITY: usize = 122;
pub const SCHED_GETAFFINITY: usize = 123;
pub const SCHED_YIELD: usize = 124;
pub const SCHED_GET_PRIORITY_MAX: usize = 125;
pub const SCHED_GET_PRIORITY_MIN: usize = 126;
pub const SCHED_RR_GET_INTERVAL: usize = 423;
pub const RESTART_SYSCALL: usize = 128;
pub const KILL: usize = 129;
pub const TKILL: usize = 130;
pub const TGKILL: usize = 131;
pub const SIGALTSTACK: usize = 132;
pub const RT_SIGSUSPEND: usize = 133;
pub const RT_SIGACTION: usize = 134;
pub const RT_SIGPROCMASK: usize = 135;
pub const RT_SIGPENDING: usize = 136;
pub const RT_SIGTIMEDWAIT_TIME32: usize = 137;
pub const RT_SIGQUEUEINFO: usize = 138;
pub const SETPRIORITY: usize = 140;
pub const GETPRIORITY: usize = 141;
pub const REBOOT: usize = 142;
pub const SETREGID: usize = 143;
pub const SETGID: usize = 144;
pub const SETREUID: usize = 145;
pub const SETUID: usize = 146;
pub const SETRESUID: usize = 147;
pub const GETRESUID: usize = 148;
pub const SETRESGID: usize = 149;
pub const GETRESGID: usize = 150;
pub const SETFSUID: usize = 151;
pub const SETFSGID: usize = 152;
pub const TIMES: usize = 153;
pub const SETPGID: usize = 154;
pub const GETPGID: usize = 155;
pub const GETSID: usize = 156;
pub const SETSID: usize = 157;
pub const GETGROUPS: usize = 158;
pub const SETGROUPS: usize = 159;
pub const NEWUNAME: usize = 160;
pub const SETHOSTNAME: usize = 161;
pub const SETDOMAINNAME: usize = 162;
pub const GETRLIMIT: usize = 163;
pub const SETRLIMIT: usize = 164;
pub const GETRUSAGE: usize = 165;
pub const UMASK: usize = 166;
pub const PRCTL: usize = 167;
pub const GETCPU: usize = 168;
pub const GETTIMEOFDAY: usize = 169;
pub const SETTIMEOFDAY: usize = 170;
pub const ADJTIMEX: usize = 171;
pub const GETPID: usize = 172;
pub const GETPPID: usize = 173;
pub const GETUID: usize = 174;
pub const GETEUID: usize = 175;
pub const GETGID: usize = 176;
pub const GETEGID: usize = 177;
pub const GETTID: usize = 178;
pub const SYSINFO: usize = 179;
pub const MQ_OPEN: usize = 180;
pub const MQ_UNLINK: usize = 181;
pub const MQ_TIMEDSEND: usize = 418;
pub const MQ_TIMEDRECEIVE: usize = 419;
pub const MQ_NOTIFY: usize = 184;
pub const MQ_GETSETATTR: usize = 185;
pub const MSGGET: usize = 186;
pub const MSGCTL: usize = 187;
pub const MSGRCV: usize = 188;
pub const MSGSND: usize = 189;
pub const SEMGET: usize = 190;
pub const SEMCTL: usize = 191;
pub const SEMTIMEDOP: usize = 420;
pub const SEMOP: usize = 193;
pub const SHMGET: usize = 194;
pub const SHMCTL: usize = 195;
pub const SHMAT: usize = 196;
pub const SHMDT: usize = 197;
pub const SOCKET: usize = 198;
pub const SOCKETPAIR: usize = 199;
pub const BIND: usize = 200;
pub const LISTEN: usize = 201;
pub const ACCEPT: usize = 202;
pub const CONNECT: usize = 203;
pub const GETSOCKNAME: usize = 204;
pub const GETPEERNAME: usize = 205;
pub const SENDTO: usize = 206;
pub const RECVFROM: usize = 207;
pub const SETSOCKOPT: usize = 208;
pub const GETSOCKOPT: usize = 209;
pub const SHUTDOWN: usize = 210;
pub const SENDMSG: usize = 211;
pub const RECVMSG: usize = 212;
pub const READAHEAD: usize = 213;
pub const BRK: usize = 214;
pub const MUNMAP: usize = 215;
pub const MREMAP: usize = 216;
pub const ADD_KEY: usize = 217;
pub const REQUEST_KEY: usize = 218;
pub const KEYCTL: usize = 219;
pub const CLONE: usize = 220;
pub const EXECVE: usize = 221;
pub const MMAP: usize = 222;
pub const FADVISE64_64: usize = 223;
pub const SWAPON: usize = 224;
pub const SWAPOFF: usize = 225;
pub const MPROTECT: usize = 226;
pub const MSYNC: usize = 227;
pub const MLOCK: usize = 228;
pub const MUNLOCK: usize = 229;
pub const MLOCKALL: usize = 230;
pub const MUNLOCKALL: usize = 231;
pub const MINCORE: usize = 232;
pub const MADVISE: usize = 233;
pub const REMAP_FILE_PAGES: usize = 234;
pub const MBIND: usize = 235;
pub const GET_MEMPOLICY: usize = 236;
pub const SET_MEMPOLICY: usize = 237;
pub const MIGRATE_PAGES: usize = 238;
pub const MOVE_PAGES: usize = 239;
pub const RT_TGSIGQUEUEINFO: usize = 240;
pub const PERF_EVENT_OPEN: usize = 241;
pub const ACCEPT4: usize = 242;
pub const RECVMMSG_TIME32: usize = 243;
pub const WAIT4: usize = 260;
pub const PRLIMIT64: usize = 261;
pub const FANOTIFY_INIT: usize = 262;
pub const FANOTIFY_MARK: usize = 263;
pub const NAME_TO_HANDLE_AT: usize = 264;
pub const OPEN_BY_HANDLE_AT: usize = 265;
pub const CLOCK_ADJTIME: usize = 405;
pub const SYNCFS: usize = 267;
pub const SETNS: usize = 268;
pub const SENDMMSG: usize = 269;
pub const PROCESS_VM_READV: usize = 270;
pub const PROCESS_VM_WRITEV: usize = 271;
pub const KCMP: usize = 272;
pub const FINIT_MODULE: usize = 273;
pub const SCHED_SETATTR: usize = 274;
pub const SCHED_GETATTR: usize = 275;
pub const RENAMEAT2: usize = 276;
pub const SECCOMP: usize = 277;
pub const GETRANDOM: usize = 278;
pub const MEMFD_CREATE: usize = 279;
pub const BPF: usize = 280;
pub const EXECVEAT: usize = 281;
pub const USERFAULTFD: usize = 282;
pub const MEMBARRIER: usize = 283;
pub const MLOCK2: usize = 284;
pub const COPY_FILE_RANGE: usize = 285;
pub const PREADV2: usize = 286;
pub const PWRITEV2: usize = 287;
pub const PKEY_MPROTECT: usize = 288;
pub const PKEY_ALLOC: usize = 289;
pub const PKEY_FREE: usize = 290;
pub const STATX: usize = 291;
pub const IO_PGETEVENTS: usize = 416;
pub const RSEQ: usize = 293;
pub const KEXEC_FILE_LOAD: usize = 294;
pub const PIDFD_SEND_SIGNAL: usize = 424;
pub const IO_URING_SETUP: usize = 425;
pub const IO_URING_ENTER: usize = 426;
pub const IO_URING_REGISTER: usize = 427;
pub const OPEN_TREE: usize = 428;
pub const MOVE_MOUNT: usize = 429;
pub const FSOPEN: usize = 430;
pub const FSCONFIG: usize = 431;
pub const FSMOUNT: usize = 432;
pub const FSPICK: usize = 433;
pub const PIDFD_OPEN: usize = 434;
pub const CLONE3: usize = 435;
pub const CLOSE_RANGE: usize = 436;
pub const OPENAT2: usize = 437;
pub const PIDFD_GETFD: usize = 438;
pub const FACCESSAT2: usize = 439;
pub const PROCESS_MADVISE: usize = 440;
