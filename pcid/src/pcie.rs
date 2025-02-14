use pci_types::{ConfigRegionAccess, PciAddress};
use rstd::{alloc::vec::Vec, mm::PhysBorrowed};
use spin::mutex::Mutex;

use crate::pci_fallback::Pci;

pub struct InterruptMap {
    pub addr: [u32; 3],
    pub interrupt: u32,
    pub parent_phandle: u32,
    pub parent_interrupt: [u32; 3],
    pub parent_interrupt_cells: usize,
}

pub const MCFG_NAME: [u8; 4] = *b"MCFG";

#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct Mcfg {
    // base sdt fields
    name: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: u32,
    _rsvd: [u8; 8],
}
unsafe impl plain::Plain for Mcfg {}

/// The "Memory Mapped Enhanced Configuration Space Base Address Allocation Structure" (yes, it's
/// called that).
#[repr(packed)]
#[derive(Clone, Copy, Debug)]
pub struct PcieAlloc {
    pub base_addr: u64,
    pub seg_group_num: u16,
    pub start_bus: u8,
    pub end_bus: u8,
    _rsvd: [u8; 4],
}
unsafe impl plain::Plain for PcieAlloc {}

#[derive(Debug)]
struct PcieAllocs<'a>(&'a [PcieAlloc]);

impl Mcfg {
    fn with<T>(
        f: impl FnOnce(PcieAllocs<'_>, Vec<InterruptMap>, [u32; 4]) -> Result<T, ()>,
    ) -> Result<T, ()> {
        let mut fd = usize::MAX;
        while fd == usize::MAX {
            fd = rstd::fs::open(":acpi:tables", 0) as usize;

            rstd::proc::r#yield();
        }

        let info_list = rstd::fs::list_dir(fd);

        for table_direntry in info_list {
            let table_filename = table_direntry.name.as_str().as_bytes();
            if table_filename.get(0..4) == Some(&MCFG_NAME) {
                let mcfg_fd = rstd::fs::open(
                    rstd::alloc::format!(":acpi:tables:{}", table_direntry.name.as_str()).as_str(),
                    0,
                ) as usize;
                let mut stat = rstd::stat::Stat::default();
                rstd::fs::fstat(mcfg_fd, stat.as_mut_ptr() as usize);
                let mcfg_fsize = stat.st_size;
                println!("MCFG table size: {}", mcfg_fsize);

                let mut bytes = rstd::alloc::vec![0u8; mcfg_fsize as usize];
                rstd::fs::read(mcfg_fd, bytes.as_mut_ptr() as usize, bytes.len());

                match Mcfg::parse(&bytes) {
                    Some((mcfg, allocs)) => {
                        println!("MCFG: {mcfg:?} ALLOCS {allocs:?}");
                        return f(allocs, Vec::new(), [u32::MAX; 4]);
                    }
                    None => {
                        return Err(());
                    }
                }
            }
        }

        Err(())
    }

    fn parse<'a>(bytes: &'a [u8]) -> Option<(&'a Mcfg, PcieAllocs<'a>)> {
        if bytes.len() < core::mem::size_of::<Mcfg>() {
            return None;
        }
        let (header_bytes, allocs_bytes) = bytes.split_at(core::mem::size_of::<Mcfg>());

        let mcfg =
            plain::from_bytes::<Mcfg>(header_bytes).expect("packed -> align 1, checked size");
        if mcfg.length as usize != bytes.len() {
            println!("MCFG {mcfg:?} length mismatch, expected {}", bytes.len());
            return None;
        }
        // TODO: Allow invalid bytes not divisible by PcieAlloc?

        let allocs_len = allocs_bytes.len() / core::mem::size_of::<PcieAlloc>()
            * core::mem::size_of::<PcieAlloc>();

        let allocs = plain::slice_from_bytes::<PcieAlloc>(&allocs_bytes[..allocs_len])
            .expect("packed -> align 1, checked size");
        Some((mcfg, PcieAllocs(allocs)))
    }
}

pub struct Pcie {
    lock: Mutex<()>,
    allocs: Vec<Alloc>,
    pub interrupt_map: Vec<InterruptMap>,
    pub interrupt_map_mask: [u32; 4],
    fallback: Pci,
}
struct Alloc {
    seg: u16,
    start_bus: u8,
    end_bus: u8,
    mem: PhysBorrowed,
}

unsafe impl Send for Pcie {}
unsafe impl Sync for Pcie {}

const BYTES_PER_BUS: usize = 1 << 20;

impl Pcie {
    pub fn new() -> Self {
        match Mcfg::with(Self::from_allocs) {
            Ok(pcie) => pcie,
            Err(acpi_error) => {
                panic!(
                    "Couldn't retrieve PCIe info, perhaps the kernel is not compiled with \
                    acpi or device tree support? Using the PCI 3.0 configuration space \
                    instead. ACPI error: {:?}",
                    acpi_error
                );
            }
        }
    }

    fn from_allocs(
        allocs: PcieAllocs<'_>,
        interrupt_map: Vec<InterruptMap>,
        interrupt_map_mask: [u32; 4],
    ) -> Result<Pcie, ()> {
        let mut allocs = allocs
            .0
            .iter()
            .filter_map(|desc| {
                Some(Alloc {
                    seg: desc.seg_group_num,
                    start_bus: desc.start_bus,
                    end_bus: desc.end_bus,
                    mem: PhysBorrowed::map(
                        desc.base_addr.try_into().ok()?,
                        BYTES_PER_BUS
                            * (usize::from(desc.end_bus) - usize::from(desc.start_bus) + 1),
                    )
                    .inspect_err(|_err| {
                        println!(
                            "failed to map seg {} bus {}..={}",
                            { desc.seg_group_num },
                            { desc.start_bus },
                            { desc.end_bus },
                        )
                    })
                    .ok()?,
                })
            })
            .collect::<Vec<_>>();

        allocs.sort_by_key(|alloc| (alloc.seg, alloc.start_bus));

        Ok(Self {
            lock: Mutex::new(()),
            allocs,
            interrupt_map,
            interrupt_map_mask,
            fallback: Pci::new(),
        })
    }

    fn bus_addr(&self, seg: u16, bus: u8) -> Option<*mut u32> {
        let alloc = match self
            .allocs
            .binary_search_by_key(&(seg, bus), |alloc| (alloc.seg, alloc.start_bus))
        {
            Ok(present_idx) => &self.allocs[present_idx],
            Err(0) => return None,
            Err(above_idx) => {
                let below_alloc = &self.allocs[above_idx - 1];
                if bus > below_alloc.end_bus {
                    return None;
                }
                below_alloc
            }
        };
        let bus_off = bus - alloc.start_bus;
        Some(unsafe {
            alloc
                .mem
                .as_ptr()
                .cast::<u8>()
                .add(usize::from(bus_off) * BYTES_PER_BUS)
                .cast::<u32>()
        })
    }

    fn bus_addr_offset_in_dwords(address: PciAddress, offset: u16) -> usize {
        assert_eq!(offset & 0xFFFC, offset, "pcie offset not dword-aligned");
        assert_eq!(offset & 0x0FFF, offset, "pcie offset larger than 4095");

        (((address.device() as usize) << 15)
            | ((address.function() as usize) << 12)
            | (offset as usize))
            >> 2
    }
    // TODO: A safer interface, using e.g. a VolatileCell or Volatile<'a>. The PhysBorrowed wrapper
    // can possibly deref to or provide a Volatile<T>.
    fn mmio_addr(&self, address: PciAddress, offset: u16) -> Option<*mut u32> {
        assert_eq!(
            address.segment(),
            0,
            "multiple segments not yet implemented"
        );

        let bus_addr = self.bus_addr(address.segment(), address.bus())?;
        Some(unsafe { bus_addr.add(Self::bus_addr_offset_in_dwords(address, offset)) })
    }
}

impl ConfigRegionAccess for Pcie {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let _guard = self.lock.lock();

        match self.mmio_addr(address, offset) {
            Some(addr) => unsafe { addr.read_volatile() },
            None => unsafe { self.fallback.read(address, offset) },
        }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let _guard = self.lock.lock();

        match self.mmio_addr(address, offset) {
            Some(addr) => unsafe { addr.write_volatile(value) },
            None => unsafe { self.fallback.write(address, offset, value) },
        }
    }
}
