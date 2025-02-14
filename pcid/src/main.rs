#![no_std]
#![no_main]

use core::usize;

#[macro_use]
extern crate rstd;

pub const MCFG_NAME: [u8; 4] = *b"MCFG";

#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    println!("PCID starting...");

    let mut fd = usize::MAX;
    while fd == usize::MAX {
        fd = rstd::fs::open(":acpi:tables", 0) as usize;

        rstd::proc::r#yield();
    }

    let info_list = rstd::fs::list_dir(fd);

    for table_direntry in info_list {
        let table_filename = table_direntry.name.as_str().as_bytes();
        if table_filename.get(0..4) == Some(&MCFG_NAME) {
            let mcfg_fd = rstd::fs::open(
                rstd::alloc::format!(":acpi:tables:{}", table_direntry.name.as_str()).as_str(),
                0,
            ) as usize;
            let mut stat = rstd::stat::Stat::default();
            rstd::fs::fstat(mcfg_fd, stat.as_mut_ptr() as usize);
            let mcfg_fsize = stat.st_size;
            println!("MCFG table size: {}", mcfg_fsize);

            let mut bytes = rstd::alloc::vec![0u8; mcfg_fsize as usize];
            rstd::fs::read(mcfg_fd, bytes.as_mut_ptr() as usize, bytes.len());
        }
    }

    loop {
        core::hint::spin_loop();
    }
}
