use nvme::{memory::Allocator, nvme::NvmeDevice};
use rstd::{alloc::vec::Vec, println};

use crate::fs::NvmeFS;

pub struct NvmeAllocator;

impl Allocator for NvmeAllocator {
    unsafe fn allocate(&self, size: usize) -> (usize, usize) {
        let (phys, virt) = rstd::dma::DmaManager::allocate(size);
        (phys.as_u64() as usize, virt.as_u64() as usize)
    }
}

pub fn init() -> NvmeFS {
    let mut nvme_devices = Vec::new();
    let mut nvme_lens = Vec::new();

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

    let hd_size = namespace.1;

    nvme_devices.push(nvme_device);
    nvme_lens.push(hd_size as usize);

    let fs = NvmeFS::new(nvme_devices, nvme_lens);
    return fs;
}
