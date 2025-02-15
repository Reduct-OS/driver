#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(inherent_str_constructors)]

use dev::DevInode;
use gpt_parser::{PARTITIONS, parse_gpt_disk};
use inode::{ROOT, user_open};
use rstd::println;

extern crate rstd;

mod dev;
mod fat32;
mod gpt_parser;
mod inode;
// mod test_log;

fn try_open_root_device() -> usize {
    let mut fd = usize::MAX;
    while fd == usize::MAX {
        fd = rstd::fs::open(":block:nvme:0", 0) as usize;
        // todo: 支持更多的设备类型

        rstd::proc::r#yield();
    }

    return fd;
}

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("fsmd starting...");

    // test_log::init();

    let fd = try_open_root_device();
    println!("root device fd: {}", fd);

    let root_device = DevInode::new(fd);
    parse_gpt_disk(root_device).expect("Cannot parse GPT disk");

    println!("Parse GPT disk OK");

    let partition = PARTITIONS
        .lock()
        .iter()
        .next()
        .cloned()
        .expect("No GPT partition at root device");

    // todo: 支持多文件系统
    let fsroot = fat32::Fat32Volume::new(partition).expect("Cannot open FAT volume");

    *ROOT.lock() = fsroot;

    println!("fsmd OK");

    loop {
        core::hint::spin_loop();
    }
}
