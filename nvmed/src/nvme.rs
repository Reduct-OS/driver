use nvme::{memory::Allocator, nvme::NvmeDevice};
use rstd::{alloc::vec::Vec, println};
use spin::Mutex;

pub struct NvmeAllocator;

impl Allocator for NvmeAllocator {
    unsafe fn allocate(&self, size: usize) -> (usize, usize) {
        let (phys, virt) = rstd::dma::DmaManager::allocate(size);
        (phys.as_u64() as usize, virt.as_u64() as usize)
    }
}

pub static NVME_DEVICES: Mutex<Vec<NvmeDevice<NvmeAllocator>>> = Mutex::new(Vec::new());

pub fn init() {
    let mut fd = usize::MAX;
    while fd == usize::MAX {
        fd = rstd::fs::open(":pci:nvme:0", 0) as usize;

        rstd::proc::r#yield();
    }

    let mut stat = rstd::stat::Stat::default();
    rstd::fs::fstat(fd, stat.as_mut_ptr() as usize);
    let bar_fsize = stat.st_size;
    println!("bar size: {}", bar_fsize);

    let buffer = rstd::fs::ioctl(fd, 1, 0) as usize;
    rstd::mm::physmap(buffer, buffer, bar_fsize as usize);

    let mut nvme_device = NvmeDevice::init(buffer, bar_fsize as usize, NvmeAllocator)
        .expect("Failed to init NVMe device");

    nvme_device
        .identify_controller()
        .expect("Failed to identify NVMe controller");

    let list = nvme_device.identify_namespace_list(0);
    println!("Namespace list: {:?}", list);

    let namespace = nvme_device.identify_namespace(1);
    println!("Namespace: {:?}", namespace);

    NVME_DEVICES.lock().push(nvme_device);
}

pub fn read_block(idx: usize, lba: u64, buffer: &mut [u8]) {
    let mut nvme_devices = NVME_DEVICES.lock();
    let nvme_device = nvme_devices.get_mut(idx).unwrap();
    nvme_device.read_copied(buffer, lba).unwrap();
}

pub fn write_block(idx: usize, lba: u64, buffer: &mut [u8]) {
    let mut nvme_devices = NVME_DEVICES.lock();
    let nvme_device = nvme_devices.get_mut(idx).unwrap();
    nvme_device.write_copied(buffer, lba).unwrap();
}
