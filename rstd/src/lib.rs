#![no_std]
#![no_main]
#![allow(asm_sub_register)]
#![feature(macro_metavar_expr)]

use core::usize;

pub extern crate alloc;

#[macro_use]
pub mod macros;
pub mod dma;
pub mod fs;
pub mod mm;
pub mod proc;
pub mod stat;
pub mod stdio;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    proc::exit(usize::MAX);
    loop {}
}
