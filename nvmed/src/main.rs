#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(inherent_str_constructors)]

use rstd::println;

extern crate rstd;

pub mod fs;
pub mod nvme;

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("nvmed starting...");

    let mut fs = nvme::init();

    rstd::fs::registfs("block", fs.fs_addr());

    println!("Regist nvme fs OK");

    fs.while_parse()
}
