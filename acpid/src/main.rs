#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(inherent_str_constructors)]
#![feature(vec_into_raw_parts)]

use fs::AcpiFS;
use rstd::alloc::sync::Arc;

#[macro_use]
extern crate rstd;

pub mod acpi;
mod fs;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("ACPID starting...");

    let fd = rstd::fs::open("/dev/kernel.acpi", 0) as usize;
    println!("ACPID get acpi file fd = {}", fd);

    let mut stat = rstd::stat::Stat::default();
    rstd::fs::fstat(fd, stat.as_mut_ptr() as usize);
    let acpi_fsize = stat.st_size;

    println!("ACPID get acpi file size = {}", acpi_fsize);

    let mut rxsdt_buf = rstd::alloc::vec![0u8; acpi_fsize as usize];
    rstd::fs::read(fd, rxsdt_buf.as_mut_ptr() as usize, acpi_fsize as usize);

    let rxsdt_raw_data: Arc<[u8]> = Arc::from(rxsdt_buf.as_slice());
    let sdt = self::acpi::Sdt::new(rxsdt_raw_data).expect("acpid: failed to parse [RX]SDT");

    let mut thirty_two_bit;
    let mut sixty_four_bit;

    let physaddrs_iter = match &sdt.signature {
        b"RSDT" => {
            thirty_two_bit = sdt
                .data()
                .chunks(core::mem::size_of::<u32>())
                // TODO: With const generics, the compiler has some way of doing this for static sizes.
                .map(|chunk| <[u8; core::mem::size_of::<u32>()]>::try_from(chunk).unwrap())
                .map(|chunk| u32::from_le_bytes(chunk))
                .map(u64::from);

            &mut thirty_two_bit as &mut dyn Iterator<Item = u64>
        }
        b"XSDT" => {
            sixty_four_bit = sdt
                .data()
                .chunks(core::mem::size_of::<u64>())
                .map(|chunk| <[u8; core::mem::size_of::<u64>()]>::try_from(chunk).unwrap())
                .map(|chunk| u64::from_le_bytes(chunk));

            &mut sixty_four_bit as &mut dyn Iterator<Item = u64>
        }
        _ => panic!("acpid: expected [RX]SDT from kernel to be either of those"),
    };

    let acpi_context = self::acpi::AcpiContext::init(physaddrs_iter);

    let mut fs = AcpiFS::new(acpi_context);

    rstd::fs::registfs("acpi", fs.fs_addr());

    println!("Regist acpi fs OK");

    drop(rxsdt_buf);

    fs.while_parse()
}
