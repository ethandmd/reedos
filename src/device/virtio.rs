//! Access the virtio device through the mmio interface provided by QEMU.
//! [Virtual I/O Device (VIRTIO) Specs](https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html)

// Also a nice walkthrough: https://www.redhat.com/en/blog/virtio-devices-and-drivers-overview-headjack-and-phone

use crate::vm::request_phys_page;
use crate::alloc::vec::Vec;
use core::cell::OnceCell;

static mut BLK_DEV: OnceCell<SplitVirtQueue> = OnceCell::new();

// Also checkout: https://wiki.osdev.org/Virtio
// Define the virtio constants for MMIO.
// These values are referenced from section 4.2.2 of the virtio-v1.1 spec.
// * NOTICE *
// Since we assume virtio over mmio here, it will never be possible to do device
// discovery, we will have to know exactly where in memory the virtio device is.
// Assume that we are only interested in virtio-mmio. These values are not valid for
// other virtio transport options (over PCI bus, channel I/O).
const VIRTIO_BASE: usize = 0x10001000; // From hw/params.rs
const VIRTIO_MAGIC: usize = 0x0; //0x74726976 := Little endian equiv to "virt" string.
const VIRTIO_VERSION: usize = 0x004; // Device version number is 0x2, legace 0x1.
const VIRTIO_DEVICE_ID: usize = 0x008; // c.f. https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.pdf#b7
const VIRTIO_VENDOR_ID: usize = 0x00c;
const VIRTIO_DEVICE_FEATURES: usize = 0x010; // Flags := supported feature map. See section 2.2 of spec.
const VIRTIO_DEVICE_FEATURES_SEL: usize = 0x014; // Read above flags then write this reg with desired feats.
const VIRTIO_DRIVER_FEATURES: usize = 0x020;
const VIRTIO_DRIVER_FEATURES_SEL: usize = 0x024; // See device_*.
const VIRTIO_QUEUE_SEL: usize = 0x030; // Zero indexed queue selection for below regs:
const VIRTIO_QUEUE_NUM_MAX: usize = 0x034; // What it says on the tin.
const VIRTIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_QUEUE_READY: usize = 0x044; // Write 0x1 to tell device it can execute requests in the sel queue.
const VIRTIO_QUEUE_NOTIFY: usize = 0x050; // Tell dev there are new buffers in queue to process.
const VIRTIO_INTERRUPT_STATUS: usize = 0x060; // Read to get bit mask of causal events.
const VIRTIO_INTERRUPT_ACK: usize = 0x064;
const VIRTIO_STATUS: usize = 0x070; // Read returns dev status flags; Write sets flags.
const VIRTIO_QUEUE_DESC_LOW: usize = 0x080; // Low bits of 64bit address.
const VIRTIO_QUEUE_DESC_HIGH: usize = 0x084; // High bits. Notify dev of location of desc area of QUEUE_SEL.
const VIRTIO_QUEUE_DRIVER_LOW: usize = 0x090;
const VIRTIO_QUEUE_DRIVER_HIGH: usize = 0x094; // Same as above but notifies dev of driver area of QUEUE_SEL.
const VIRTIO_QUEUE_DEVICE_LOW: usize = 0x0a0;
const VIRTIO_QUEUE_DEVICE_HIGH: usize = 0x0a4; // Same as above. Notify of device area of QUEUE_SEL.
const VIRTIO_CONFIG_GENERATION: usize = 0x0fc; // Config atomocity value. Use to access config space.
const VIRTIO_CONFIG: usize = 0x100; // 0x100+; Dev specific config starts here.

// Device Status; Section 2.1.
// Indicates completed steps of initialization sequence.
// Never clear, only set bits as steps completed during init.
enum VirtioDeviceStatus {
    Ack = 1, // Found and recognize the device.
    Driver = 2, // Know how to drive the device.
    DriverOk = 4, // Driver is ready to drive the device.
    FeaturesOk = 8, // Driver has ACK'd all the features it knows; feature negotiation complete.
    DeviceNeedsReset = 0x40, // Unrecoverable error.
    Failed = 0x80, // Internal error, driver rejected device, device fatal.
}

// Device Features; Section 5.2.3.
// Select \subseteq of features the device offers.
// Set FeaturesOk flag once feature negotiation is done.
// Feature bits 0-23 specific to device type.
// bits 24-37 reserved.
// bits 38+ reserved.
const VIRTIO_BLK_F_BARRIER: u32 = 0; // legacy
const VIRTIO_BLK_F_SIZE_MAX: u32 = 1;
const VIRTIO_BLK_F_SEG_MAX: u32 = 2;
const VIRTIO_BLK_F_GEOMETRY: u32 = 4;
const VIRTIO_BLK_F_RO: u32 = 5;
const VIRTIO_BLK_F_BLK_SIZE: u32 = 6;
const VIRTIO_BLK_F_SCSI: u32 = 7;   // legacy
const VIRTIO_BLK_F_FLUSH: u32 = 9;
const VIRTIO_BLK_F_TOPOLOGY: u32 = 10;
const VIRTIO_BLK_F_CONFIG_WCE: u32 = 11; // Dev can toggle (write through : write back) cache.
const VIRTIO_BLK_F_MQ: u32 = 12;
const VIRTIO_BLK_F_DISCARD: u32 = 13;
const VIRTIO_BLK_F_WRITE_ZEROES: u32 = 14;
const VIRTIO_BLK_F_ANY_LAYOUT: u32 = 27;
const VIRTIO_RING_F_EVENT_IDX: u32 = 28;
const VIRTIO_RING_F_INDIRECT_DESC: u32 = 29;

// Clear these bits during feat negotiation.
static DEVICE_FEATURE_CLEAR: [u32; 7] = [
    VIRTIO_BLK_F_RO,
    VIRTIO_BLK_F_SCSI,
    VIRTIO_BLK_F_WRITE_ZEROES,
    VIRTIO_BLK_F_MQ,
    VIRTIO_BLK_F_ANY_LAYOUT,
    VIRTIO_RING_F_EVENT_IDX,
    VIRTIO_RING_F_INDIRECT_DESC,
];

// Block request types
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 =  1;
const VIRTIO_BLK_T_FLUSH: u32 = 4;
const VIRTIO_BLK_T_DISCARD: u32 = 11;
const VIRTIO_BLK_T_WRITE_ZEROES: u32 = 13;

// Block request status
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

// VirtQueues; Section 2.5.
// 
// Based on (legacy supported) splitqueue: Section 2.6.
// Device versions <= 0x1 only have split queue.
struct SplitVirtQueue {
    // Descriptor Area: describe buffers (make fixed array?)
    desc: Vec<VirtQueueDesc>,
    // Driver Area (aka Available ring): extra info from driver to device
    avail: Vec<VirtQueueAvail>,
    // Device Area (aka Used ring): extra info from device to driver
    // * NEED PADDING HERE???? *
    // pad: Vec<u8>,
    used: Vec<VirtQueueUsed>,
}

impl SplitVirtQueue {
    // Ptr's must have been allocated with global alloc.
    fn new(
        desc_ptr: *mut usize,
        avail_ptr: *mut usize,
        used_ptr: *mut usize,
        ) -> Self {
        let (len, cap) = (0x1000, 0x1000);
        let desc = unsafe { Vec::from_raw_parts(desc_ptr as *mut VirtQueueDesc, len, cap) };
        let avail = unsafe { Vec::from_raw_parts(avail_ptr as *mut VirtQueueAvail, len, cap) };
        let used = unsafe { Vec::from_raw_parts(used_ptr as *mut VirtQueueUsed, len, cap) };
        Self { desc, avail, used }
    }
}

// VirtQueue Descriptor Table; Section 2.6.5.
// Everything little endian.
// * If flag is empty => read-only buffer. *
enum VirtQueueDescFeat {
    Next = 0x1,       // Buffer continues into NEXT field.
    Write = 0x2,      // Buffer as device write-only.
    Indirect = 0x4,   // Buffer contains a list of buffer descriptors.
}

// Note that we don't need IOMMU since this is all in QEMU process.
// If this were a real physical device, then we need IOMMU.
#[repr(C)]
struct VirtQueueDesc {
    addr: u64, // Specifically little endian 64
    len: u32,
    flags: u16,
    next: u16,
}

const RING_SIZE: usize = 2; // Power of 2.

// Section 2.6.6
// ** Ring queue size is power of 2 and avail, used
// queues should be same size.
#[repr(C)]
struct VirtQueueAvail {
    flags: u16,             // LSB := VIRTQ_AVAIL_F_NO_INTERRUPT
    idx: u16,               // Where driver puts next desc entry % queue size.
    ring: [u16; RING_SIZE],  // Length := numb o chain heads
    used_event: u16,        // Only if feature EVENT_INDEX is set.
}

// Section 2.6.8
#[repr(C)]
struct VirtQueueUsed {
    flags: u16,
    idx: u16,
    used_ring: [VirtQueueUsedElem; RING_SIZE],
    avail_event: u16, // Only if feature EVENT_INDEX is set.
}

#[repr(C)]
struct VirtQueueUsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
struct VirtBlkReq {
    flavor: u32, // BLK_T_IN, BLK_T_OUT, ..
    reserved: u32,
    sector: u64,
    data: Vec<u8>, // We'll see how this ages
    status: u8, // BLK_S_OK, ...
}

fn read_virtio_reg_4(offset: usize) -> u32 {
    unsafe {
        ((VIRTIO_BASE + offset) as *mut u32).read_volatile()
    }
}

fn write_virtio_reg_4(offset: usize, data: u32) {
    let ptr = (VIRTIO_BASE + offset) as *mut u32;
    println!("Writing addr: {:?} with: {:#02x}", ptr, data);
    unsafe {
        ptr.write_volatile(data)
    }
}

// Device Initialization: Sections 3.1 (general) + 4.2.3 (mmio)
// Currently testing this QEMU board device:
pub fn virtio_init() -> Result<(), &'static str> {
    // Step 0: Read device info.
    let magic = read_virtio_reg_4(VIRTIO_MAGIC);
    let ver = read_virtio_reg_4(VIRTIO_VERSION);
    let dev_id = read_virtio_reg_4(VIRTIO_DEVICE_ID);
    let ven_id = read_virtio_reg_4(VIRTIO_VENDOR_ID);
    if magic != 0x74726976 || ver != 0x1 || dev_id != 0x2 || ven_id != 0x554d4551 {
        return Err("Device info is incompatible.");
    }

    let mut device_status = 0x0;

    // Step 1: Reset device.
    write_virtio_reg_4(VIRTIO_STATUS, device_status);

    // Step 2: Ack device.
    device_status |= VirtioDeviceStatus::Ack as u32;
    write_virtio_reg_4(VIRTIO_STATUS, device_status);

    // Step 3: Driver status bit.
    device_status |= VirtioDeviceStatus::Driver as u32;
    write_virtio_reg_4(VIRTIO_STATUS, device_status);

    // Step 4,5,6: Negotiate features. (Conflating steps btwn new & legacy spec)
    let mut device_feature = read_virtio_reg_4(VIRTIO_DEVICE_FEATURES);
    for feat in DEVICE_FEATURE_CLEAR {
        device_feature &= !(1 << feat);
    }
    write_virtio_reg_4(VIRTIO_DEVICE_FEATURES, device_feature);
    // write feature_ok ? legacy device ver 0x1.
    device_status |= VirtioDeviceStatus::FeaturesOk as u32;
    write_virtio_reg_4(VIRTIO_STATUS, device_status);
    let new_status = read_virtio_reg_4(VIRTIO_STATUS);
    if (new_status & (VirtioDeviceStatus::FeaturesOk as u32)) == 0x0 {
        println!("FeaturesOK (not supported || not accepted).");
    }

    // Step 7: Set up virt queues; Section 4.2.3.2
    // i. Select queue and write index to QUEUE_SEL.
    write_virtio_reg_4(VIRTIO_QUEUE_SEL, 0);
    
    // ii. Check if queue in use; read QueueReady, expect 0x0.
    if read_virtio_reg_4(VIRTIO_QUEUE_READY) != 0x0 {
        return Err("Selected Queue already in use.");
    }

    // iii. Check max queue size; read QueueNumMax, if 0x0, queue not avail.
    let vq_max = read_virtio_reg_4(VIRTIO_QUEUE_NUM_MAX);
    log!(Debug, "Virtio BLK dev max queues: {}", vq_max);
    if vq_max == 0x0 || (vq_max as usize) < RING_SIZE {
        return Err("Queue is not available.");
    }

    // iv. Allocate and zero queue. Must by physically contiguous.
    let (desc_ptr, avail_ptr, used_ptr): (*mut usize, *mut usize, *mut usize);
    if let Ok(buf) = request_phys_page(3) {
        desc_ptr = buf.start();
        avail_ptr = unsafe { desc_ptr.byte_add(0x1000) };
        used_ptr = unsafe { avail_ptr.byte_add(0x1000) };
        unsafe {
            match BLK_DEV.set(SplitVirtQueue::new(desc_ptr, avail_ptr, used_ptr)) {
                Ok(_) => {},
                Err(_) => { return Err("Could not configure global BLK_DEV with 3 queues."); },
            }
        }
    } else {
        return Err("Could not allocated sufficient memory.");
    }

    // v. Notife the device about queue size; write to QueueNum.
    write_virtio_reg_4(VIRTIO_QUEUE_NUM, RING_SIZE as u32);

    // vi. Write queue addrs to desc{high/low}, ...
    write_virtio_reg_4(VIRTIO_QUEUE_DESC_LOW, desc_ptr.addr() as u32);
    write_virtio_reg_4(VIRTIO_QUEUE_DESC_HIGH, (desc_ptr.addr() >> 32) as u32);
    write_virtio_reg_4(VIRTIO_QUEUE_DRIVER_LOW, avail_ptr as u32);
    write_virtio_reg_4(VIRTIO_QUEUE_DRIVER_HIGH, avail_ptr.map_addr(|addr| addr >> 32) as u32);
    write_virtio_reg_4(VIRTIO_QUEUE_DEVICE_LOW, used_ptr as u32);
    write_virtio_reg_4(VIRTIO_QUEUE_DEVICE_HIGH, used_ptr.map_addr(|addr| addr >> 32) as u32);

    // vii. Write 0x1 to QueueReady
    write_virtio_reg_4(VIRTIO_QUEUE_READY, 0x1);

    // Step 8: Set DriverOk bit in Device status.
    device_status |= VirtioDeviceStatus::DriverOk as u32;
    write_virtio_reg_4(VIRTIO_STATUS, device_status);
    Ok(())
}
