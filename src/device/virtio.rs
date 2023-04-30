//! Access the virtio device through the mmio interface provided by QEMU.
//! [Virtual I/O Device (VIRTIO) Specs](https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html)

// Also a nice walkthrough: https://www.redhat.com/en/blog/virtio-devices-and-drivers-overview-headjack-and-phone

use crate::alloc::vec::Vec;

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

// Device Features; Section 2.2.
// Select \subseteq of features the device offers.
// Set FeaturesOk flag once feature negotiation is done.
// Feature bits 0-23 specific to device type.
// bits 24-37 reserved.
// bits 38+ reserved.
enum VirtioDeviceFeatures {}

// VirtQueues; Section 2.5.
// 
// Based on (legacy) splitqueue: Section 2.6.
// Device versions <= 0x1 only have split queue.
struct SplitVirtQueue {
    num: u32,
    // Descriptor Area: describe buffers (make fixed array?)
    desc: Vec<VirtQueueDesc>,
    // Driver Area (aka Available ring): extra info from driver to device
    avail: VirtQueueAvailable,
    // Device Area (aka Used ring): extra info from device to driver
    // * NEED PADDING HERE???? *
    // pad: Vec<u8>,
    used: VirtQueueUsed,
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
struct VirtQueueDesc {
    addr: u64, // Specifically little endian 64
    len: u32,
    flags: u16,
    next: u16,
}

const RING_SIZE: usize = 2;
// Section 2.6.6
// ** Ring queue size is power of 2 and avail, used
// queues should be same size.
struct VirtQueueAvailable {
    flags: u16,             // LSB := VIRTQ_AVAIL_F_NO_INTERRUPT
    idx: u16,               // Where driver puts next desc entry % queue size.
    ring: [u16; RING_SIZE],  // Length := numb o chain heads
    used_event: u16,        // Only if feature EVENT_INDEX is set.
}

// Section 2.6.8
struct VirtQueueUsed {
    flags: u16,
    idx: u16,
    used_ring: [VirtQueueUsedElem; RING_SIZE],
    avail_event: u16, // Only if feature EVENT_INDEX is set.
}

struct VirtQueueUsedElem {
    id: u32,
    len: u32,
}
// Device Initialization: Sections 3 (general) + 4.2.3 (mmio)
// Currently testing this QEMU board device:
pub unsafe fn test_virtio_init() {
    //let base = VIRTIO_BASE as *mut u8;
    // Step 0: Read device info.
    let magic = ((VIRTIO_BASE + VIRTIO_MAGIC) as *mut u32).read_volatile();
    let ver = ((VIRTIO_BASE + VIRTIO_VERSION) as *mut u32).read_volatile();
    let dev_id = ((VIRTIO_BASE + VIRTIO_DEVICE_ID) as *mut u32).read_volatile();
    let ven_id = ((VIRTIO_BASE + VIRTIO_VENDOR_ID) as *mut u32).read_volatile();
    println!("VIRTIO DEVICE INFO:\n \
    magic: {:#02x}\n \
    version: {:#02x}; Want: 0x2\n \
    device ID: {:#02x}\n \
    vendor ID: {:#02x}", magic, ver, dev_id, ven_id);
}
