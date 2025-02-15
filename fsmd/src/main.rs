#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(inherent_str_constructors)]
#![feature(vec_into_raw_parts)]

use dev::DevInode;
use fs::FsmFS;
use gpt_parser::{PARTITIONS, parse_gpt_disk};
use inode::ROOT;
use rstd::{alloc::string::ToString, println};

extern crate rstd;

mod dev;
mod fat32;
mod fs;
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
    ROOT.lock().write().when_mounted("/".to_string(), None);

    println!("fsmd OK");

    let mut fsm_fs = FsmFS::new();

    rstd::fs::registfs("fsm", fsm_fs.fs_addr());

    rstd::fs::load_driver("/usr/init");

    println!("Regist fsm fs OK");

    fsm_fs.while_parse()
}
