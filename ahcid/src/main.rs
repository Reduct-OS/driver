#![no_std]
#![no_main]
#![allow(dead_code)]

use rstd::println;

extern crate rstd;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("ahcid starting...");

    loop {
        core::hint::spin_loop();
    }
}
