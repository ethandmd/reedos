//! Kernel trap handlers.
use crate::device::{clint, plic, uart, virtio};
use crate::hw::{riscv, param};

use crate::log;

extern "C" {
    pub fn __mtrapvec();
    pub fn __strapvec();
}


// TODO currently not in use
//
// pub struct TrapFrame {
//     kpgtbl: *mut PageTable,
//     handler: *const (),
//     cause: usize,
//     retpc: usize, // Return from trap program counter value.
//     regs: [usize; 32],
// }

/// These are the cause numbers for the regular s mode handler. I don't
/// see any reason they need to be public.
///
/// TODO how can we make these generic over 32/64 bit width?
const S_EXTERN_IRQ: u64 = 0x9 | ( 1 << 63);

/// Write the supervisor trap vector to stvec register on each hart.
pub fn init() {
    riscv::write_stvec(__strapvec as usize);
}

/// Machine mode trap handler.
#[no_mangle]
pub extern "C" fn m_handler() {
    let mcause = riscv::read_mcause();

    match mcause {
        riscv::MSTATUS_TIMER => {
            // log::log!(Debug, "Machine timer interupt, hart: {}", riscv::read_mhartid());
            clint::set_mtimecmp(10_000_000);
        }
        _ => {
            log::log!(
                Warning,
                "Uncaught machine mode interupt. mcause: 0x{:x}",
                mcause
            );
            panic!();
        }
    }
}

/// Supervisor mode trap handler.
#[no_mangle]
pub extern "C" fn s_handler() {
    let cause = riscv::read_scause();

    match cause {
        S_EXTERN_IRQ => {
            s_extern()
        },
        _ => {
            log::log!(
                Warning,
                "Uncaught supervisor mode interupt. scause: 0x{:x}",
                cause
            );
            panic!()
        }
    }
}

/// Called when we get a S mode external interupt. Probably UART input
/// or virtio.
fn s_extern() {
    let irq = unsafe {
        plic::PLIC.get().expect("PLIC not initialized!").claim()
    };

    const UART_IRQ: u32 = param::UART_IRQ as u32;
    const VIRTIO_IRQ: u32 = param::VIRTIO_IRQ as u32;
    match irq {
        0 => {
            // reserved for "No interrupt" according to the
            // cookbook. Just chill I guess, I don't think we need to
            // complete it
        }
        UART_IRQ => {
            // I intentionally don't hold the lock here to
            // allow printing. Normally we shouldn't print
            // here
            let input = unsafe {
                match uart::WRITER.lock().get() {
                    Some(i) => i,
                    None => {
                        // spurious irq? just exit early
                        plic::PLIC.get().unwrap().complete(irq);
                        return
                    }
                }
            };
            log!(Info, "Got UART input: {}",
                 char::from_u32(input as u32).expect(
                     "Illformed UART input character!"
                 ));
            unsafe {
                plic::PLIC.get().unwrap().complete(irq)
            };

        },
        VIRTIO_IRQ => {
            virtio::virtio_blk_intr();
            unsafe {
                plic::PLIC.get().unwrap().complete(irq)
            };
        },
        _ => {
            panic!("Uncaught PLIC exception.")
        }
    }
}
