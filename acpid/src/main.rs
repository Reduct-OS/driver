#![no_std]
#![no_main]

#[macro_use]
extern crate rstd;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("ACPID starting...");

    let fd = rstd::fs::open("/dev/kernel.acpi", 0) as usize;
    println!("ACPID get acpi file fd = {}", fd);

    let mut stat = rstd::stat::Stat::default();
    rstd::fs::fstat(fd, stat.as_mut_ptr() as usize);
    let acpi_fsize = stat.st_size;

    println!("ACPID get acpi file size = {}", acpi_fsize);

    loop {
        core::hint::spin_loop();
    }
}
