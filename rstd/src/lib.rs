#![no_std]
#![no_main]
#![allow(asm_sub_register)]
#![feature(macro_metavar_expr)]

#[macro_use]
pub mod macros;
pub mod stdio;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
