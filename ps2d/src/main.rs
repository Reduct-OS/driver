#![no_std]
#![no_main]
#![feature(inherent_str_constructors)]

use fs::Ps2FS;
use rstd::println;

extern crate rstd;

mod fs;
mod keyboard;
mod mouse;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("ps2d starting...");

    keyboard::init();
    mouse::init();

    let mut ps2_fs = Ps2FS::new();

    rstd::fs::registfs("ps2", ps2_fs.fs_addr());

    ps2_fs.while_parse()
}
