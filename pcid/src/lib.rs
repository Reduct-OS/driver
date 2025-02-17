#![no_std]
#![no_main]

use pci_types::{Bar, MAX_BARS, PciAddress, device_type::DeviceType};
use rstd::alloc::string::String;

/// All identifying information of a PCI function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullDeviceId {
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub interface: u8,
    pub revision: u8,
}

impl FullDeviceId {
    pub fn display(&self) -> String {
        let mut string = rstd::alloc::format!(
            "{:>04X}:{:>04X} {:>02X}.{:>02X}.{:>02X}.{:>02X} {:?}",
            self.vendor_id,
            self.device_id,
            self.class,
            self.subclass,
            self.interface,
            self.revision,
            self.class,
        );
        let device_type = DeviceType::from((self.class, self.subclass));
        match device_type {
            DeviceType::LegacyVgaCompatible => string.push_str("  VGA CTL"),
            DeviceType::IdeController => string.push_str(" IDE"),
            DeviceType::SataController => match self.interface {
                0 => string.push_str(" SATA VND"),
                1 => string.push_str(" SATA AHCI"),
                _ => (),
            },
            DeviceType::UsbController => match self.interface {
                0x00 => string.push_str(" UHCI"),
                0x10 => string.push_str(" OHCI"),
                0x20 => string.push_str(" EHCI"),
                0x30 => string.push_str(" XHCI"),
                _ => (),
            },
            DeviceType::NvmeController => string.push_str(" NVME"),
            _ => (),
        }
        string
    }
}

#[derive(Debug)]
pub struct PciDevice {
    pub address: PciAddress,
    pub device_id: FullDeviceId,
    pub device_type: DeviceType,
    pub bars: [Option<Bar>; MAX_BARS],
}
