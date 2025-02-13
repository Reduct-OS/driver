#![no_std]
#![no_main]

#[macro_use]
extern crate rstd;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("ACPID starting...");

    loop {
        core::hint::spin_loop();
    }
}
