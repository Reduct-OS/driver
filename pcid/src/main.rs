#![no_std]
#![no_main]

use core::usize;

#[macro_use]
extern crate rstd;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("PCID starting...");

    let mut fd = usize::MAX;
    while fd == usize::MAX {
        fd = rstd::fs::open("/acpi", 0) as usize;
    }

    println!("Open acpi table OK.");
    let buf: &mut [u8; 5] = &mut [0; 5];
    rstd::fs::read(fd, buf.as_mut_ptr() as usize, buf.len());

    loop {
        core::hint::spin_loop();
    }
}
