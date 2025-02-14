#![no_std]
#![no_main]

use pci_types::{HeaderType, PciAddress, PciHeader, PciPciBridgeHeader};
use pcid::FullDeviceId;
use pcie::Pcie;

#[macro_use]
extern crate rstd;

pub mod pci_fallback;
pub mod pcie;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("pcid starting...");

    let pcie = Pcie::new();

    println!("PCI SG-BS:DV.F VEND:DEVI CL.SC.IN.RV");

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
                    HeaderType::Endpoint => {}
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

    loop {
        core::hint::spin_loop();
    }
}
