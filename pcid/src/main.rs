#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(let_chains)]
#![feature(inherent_str_constructors)]

use core::sync::atomic::AtomicBool;

use fs::PciFS;
use pci_types::{
    Bar, CommandRegister, EndpointHeader, HeaderType, MAX_BARS, PciAddress, PciHeader,
    PciPciBridgeHeader, capability::PciCapability, device_type::DeviceType,
};
use pcid::{FullDeviceId, PciDevice};
use pcie::Pcie;
use rstd::alloc::vec::Vec;

#[macro_use]
extern crate rstd;

pub mod fs;
pub mod pci_fallback;
pub mod pcie;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("pcid starting...");

    let pcie = Pcie::new();

    println!("PCI SG-BS:DV.F VEND:DEVI CL.SC.IN.RV");

    let mut pci_devices = Vec::new();

    // FIXME Use full ACPI for enumerating the host bridges. MCFG only describes the first
    // host bridge, while multi-processor systems likely have a host bridge for each CPU.
    // See also https://www.kernel.org/doc/html/latest/PCI/acpi-info.html
    let mut bus_nums = rstd::alloc::vec![0];
    let mut bus_i = 0;
    while bus_i < bus_nums.len() {
        let bus_num = bus_nums[bus_i];
        bus_i += 1;

        'dev: for dev_num in 0..32 {
            for func_num in 0..8 {
                let header = PciHeader::new(PciAddress::new(0, bus_num, dev_num, func_num));

                let (vendor_id, device_id) = header.id(&pcie);
                if vendor_id == 0xffff && device_id == 0xffff {
                    if func_num == 0 {
                        // println!("PCI {:>02X}:{:>02X}: no dev", bus_num, dev_num);
                        continue 'dev;
                    }

                    continue;
                }

                let (revision, class, subclass, interface) = header.revision_and_class(&pcie);
                let full_device_id = FullDeviceId {
                    vendor_id,
                    device_id,
                    class,
                    subclass,
                    interface,
                    revision,
                };

                println!("PCI {} {}", header.address(), full_device_id.display());

                match header.header_type(&pcie) {
                    HeaderType::Endpoint => {
                        let mut endpoint_header =
                            EndpointHeader::from_header(header, &pcie).unwrap();

                        let endpoint_bars = |header: &EndpointHeader| {
                            let mut bars = [None; MAX_BARS];
                            let mut skip_next = false;

                            for (index, bar_slot) in bars.iter_mut().enumerate() {
                                if skip_next {
                                    skip_next = false;
                                    continue;
                                }
                                let bar = header.bar(index as u8, &pcie);
                                if let Some(Bar::Memory64 { .. }) = bar {
                                    skip_next = true;
                                }
                                *bar_slot = bar;
                            }

                            bars
                        };

                        let bars = endpoint_bars(&endpoint_header);

                        endpoint_header.capabilities(&pcie).for_each(
                            |capability| match capability {
                                PciCapability::Msi(msi) => {
                                    msi.set_enabled(true, &pcie);
                                }
                                PciCapability::MsiX(mut msix) => {
                                    msix.set_enabled(true, &pcie);
                                }
                                _ => {}
                            },
                        );

                        endpoint_header.update_command(&pcie, |command| {
                            command
                                | CommandRegister::BUS_MASTER_ENABLE
                                | CommandRegister::IO_ENABLE
                                | CommandRegister::MEMORY_ENABLE
                        });

                        let pci_device = PciDevice {
                            address: endpoint_header.header().address(),
                            device_id: full_device_id,
                            device_type: DeviceType::from((
                                full_device_id.class,
                                full_device_id.subclass,
                            )),
                            bars: bars,
                        };

                        pci_devices.push(pci_device);
                    }
                    HeaderType::PciPciBridge => {
                        let bridge_header = PciPciBridgeHeader::from_header(header, &pcie).unwrap();
                        bus_nums.push(bridge_header.secondary_bus_number(&pcie));
                    }
                    ty => {
                        println!("pcid: unknown header type: {ty:?}");
                    }
                }
            }
        }
    }

    for pci_device in pci_devices.iter() {
        match pci_device.device_type {
            // DeviceType::SataController => {
            //     static AHCI_LOADED: AtomicBool = AtomicBool::new(false);
            //     if !AHCI_LOADED.fetch_or(true, core::sync::atomic::Ordering::SeqCst) {
            //         rstd::fs::load_driver("/drv/ahcid");
            //     }
            // }
            DeviceType::NvmeController => {
                static NVME_LOADED: AtomicBool = AtomicBool::new(false);
                if !NVME_LOADED.fetch_or(true, core::sync::atomic::Ordering::SeqCst) {
                    rstd::fs::load_driver("/drv/nvmed");
                }
            }
            _ => {}
        }
    }

    let mut fs = PciFS::new(pci_devices);

    rstd::fs::registfs("pci", fs.fs_addr());

    println!("Regist pci fs OK");

    fs.while_parse()
}
