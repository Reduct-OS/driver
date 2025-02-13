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

    let rxsdt_buf = rstd::alloc::vec![0u8; acpi_fsize as usize];
    rstd::fs::read(fd, rxsdt_buf.as_ptr() as usize, acpi_fsize as usize);

    let pipe: [usize; 2] = [0; 2];
    rstd::fs::pipe(pipe);

    let new_pid = rstd::proc::fork();
    if new_pid == 0 {
        println!("Child is running!!! ret = {}", new_pid);
        rstd::fs::close(pipe[1]);
    } else {
        println!("Parent is running!!! ret = {}", new_pid);
        rstd::fs::close(pipe[0]);
    }

    loop {
        core::hint::spin_loop();
    }
}
