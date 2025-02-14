#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]

use rstd::println;

extern crate rstd;

mod nlog;
pub mod nvme;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("nvmed starting...");

    // nlog::init();

    nvme::init();

    let buffer = &mut [0u8; 512];
    nvme::read_block(0, 1, buffer);
    println!("buffer: {:#X?}", buffer);

    loop {
        core::hint::spin_loop();
    }
}
