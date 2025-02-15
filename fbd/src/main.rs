#![no_std]
#![no_main]
#![feature(inherent_str_constructors)]

use gui::Gui;
use rstd::println;

extern crate rstd;

pub mod gui;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("fbd starting...");

    let mut gui = Gui::new();

    gui.main_loop()
}
