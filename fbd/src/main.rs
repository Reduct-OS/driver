#![no_std]
#![no_main]
#![feature(inherent_str_constructors)]

use fur::{display::Display, window::WindowBuilder};
use gui::Driver;
use rstd::println;

extern crate rstd;

pub mod gui;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("fbd starting...");

    let mut window = WindowBuilder::new(800, 600);

    window
        .title("window")
        .draw(&mut Display::new(Driver::new()));

    loop {
        core::hint::spin_loop();
    }
}
