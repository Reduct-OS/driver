#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(inherent_str_constructors)]

use fb::Driver;
use rstd::println;
use window::Window;

extern crate rstd;

pub mod fb;
pub mod window;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("fbd starting...");

    let mut window = Window::new(Driver::new());
    let window = window.set_title("test").set_size(800, 600);
    window.draw();

    loop {
        core::hint::spin_loop();
    }
}
