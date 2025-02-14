use pci_types::{ConfigRegionAccess, PciAddress};
use spin::Mutex;
use x86_64::instructions::port::Port;

pub(crate) struct Pci {
    lock: Mutex<()>,
}

impl Pci {
    pub(crate) fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }

    fn address(address: PciAddress, offset: u8) -> u32 {
        assert_eq!(
            address.segment(),
            0,
            "usage of multiple segments requires PCIe extended configuration"
        );

        assert_eq!(offset & 0xFC, offset, "pci offset is not aligned");

        0x80000000
            | (u32::from(address.bus()) << 16)
            | (u32::from(address.device()) << 11)
            | (u32::from(address.function()) << 8)
            | u32::from(offset)
    }
}

impl ConfigRegionAccess for Pci {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let _guard = self.lock.lock();

        let offset =
            u8::try_from(offset).expect("offset too large for PCI 3.0 configuration space");
        let address = Self::address(address, offset);

        unsafe {
            Port::<u32>::new(0xCF8).write(address);
        }
        unsafe { Port::<u32>::new(0xCFC).read() }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let _guard = self.lock.lock();

        let offset =
            u8::try_from(offset).expect("offset too large for PCI 3.0 configuration space");
        let address = Self::address(address, offset);

        unsafe { Port::<u32>::new(0xCF8).write(address) };
        unsafe { Port::<u32>::new(0xCFC).write(value) };
    }
}
