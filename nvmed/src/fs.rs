use nvme::nvme::NvmeDevice;
use rstd::{
    alloc::vec::Vec,
    fs::{USER_IOCTL, USER_OPEN, USER_READ, USER_SIZE, USER_WRITE, UserCommand},
    println,
};
use spin::Mutex;

use crate::nvme::NvmeAllocator;

enum NvmeHandle {
    NoHandle,
    RwHandle(usize),
}

pub struct NvmeFS {
    lock: Mutex<()>,
    nvme_devices: Vec<NvmeDevice<NvmeAllocator>>,
    nvme_lens: Vec<usize>,
    current_handle: NvmeHandle,
    user_command: UserCommand,
}

impl NvmeFS {
    pub fn new(nvme_devices: Vec<NvmeDevice<NvmeAllocator>>, nvme_lens: Vec<usize>) -> Self {
        Self {
            lock: Mutex::new(()),
            nvme_devices,
            nvme_lens,
            current_handle: NvmeHandle::NoHandle,
            user_command: UserCommand::default(),
        }
    }

    pub fn fs_addr(&mut self) -> usize {
        &mut self.user_command as *mut UserCommand as usize
    }

    pub fn while_parse(&mut self) -> ! {
        loop {
            let cmd = self.user_command.cmd;
            if cmd != 0 {
                match cmd {
                    USER_OPEN => {
                        self.open(
                            str::from_utf8(unsafe {
                                core::slice::from_raw_parts(
                                    self.user_command.buf_addr as *const u8,
                                    self.user_command.buf_size,
                                )
                            })
                            .unwrap(),
                        );
                    }
                    USER_READ => {
                        if self
                            .read(self.user_command.offset, unsafe {
                                core::slice::from_raw_parts_mut(
                                    self.user_command.buf_addr as *mut u8,
                                    self.user_command.buf_size,
                                )
                            })
                            .is_ok()
                        {
                            self.user_command.ret_val = 0;
                        } else {
                            self.user_command.ret_val = -1;
                        }
                    }
                    USER_WRITE => {
                        if self
                            .write(self.user_command.offset, unsafe {
                                core::slice::from_raw_parts(
                                    self.user_command.buf_addr as *const u8,
                                    self.user_command.buf_size,
                                )
                            })
                            .is_ok()
                        {
                            self.user_command.ret_val = 0;
                        } else {
                            self.user_command.ret_val = -1;
                        }
                    }
                    USER_SIZE => {
                        if let Ok(size) = self.size() {
                            self.user_command.ret_val = size as isize;
                        } else {
                            self.user_command.ret_val = -1;
                        }
                    }
                    USER_IOCTL => {
                        if let Ok(ret) = self.ioctl(unsafe {
                            core::slice::from_raw_parts_mut(
                                self.user_command.buf_addr as *mut usize,
                                self.user_command.buf_size,
                            )
                        }) {
                            self.user_command.ret_val = ret as isize;
                        } else {
                            self.user_command.ret_val = -1;
                        }
                    }
                    _ => println!("nvmed: unknown command: {}", cmd),
                }

                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        let _guard = self.lock.lock();

        if let Some((nvme_device, idx)) = path.split_once(":") {
            assert_eq!(nvme_device, "nvme");
            let idx = idx.parse::<usize>().unwrap();

            if idx < self.nvme_devices.len() {
                self.current_handle = NvmeHandle::RwHandle(idx);
            }
        }
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        match self.current_handle {
            NvmeHandle::RwHandle(idx) => {
                let nvme_device = &mut self.nvme_devices[idx];
                nvme_device.read_copied(buf, offset as u64 / 512).unwrap();
            }
            _ => return Err(()),
        }

        Ok(buf.len())
    }

    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        match self.current_handle {
            NvmeHandle::RwHandle(idx) => {
                let nvme_device = &mut self.nvme_devices[idx];
                nvme_device.write_copied(buf, offset as u64 / 512).unwrap();
            }
            _ => return Err(()),
        }

        Ok(buf.len())
    }

    fn size(&mut self) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        match self.current_handle {
            NvmeHandle::RwHandle(idx) => {
                if let Some(&len) = self.nvme_lens.get(idx) {
                    return Ok(len);
                }
            }
            _ => return Err(()),
        }

        Err(())
    }

    #[allow(unused_variables)]
    fn ioctl(&mut self, buf: &[usize]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        let cmd = buf[0];
        let arg = buf[1];

        Err(())
    }
}
