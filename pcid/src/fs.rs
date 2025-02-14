use pci_types::{Bar, MAX_BARS, device_type::DeviceType};
use pcid::PciDevice;
use rstd::{
    alloc::vec::Vec,
    fs::{USER_IOCTL, USER_OPEN, USER_READ, USER_SIZE, USER_WRITE, UserCommand},
};
use spin::Mutex;

enum PciHandle {
    NoHandle,
    GetBar(Bar),
}

pub struct PciFS {
    lock: Mutex<()>,
    pci_devices: Vec<PciDevice>,
    current_handle: PciHandle,
    user_command: UserCommand,
}

impl PciFS {
    pub fn new(devices: Vec<PciDevice>) -> Self {
        Self {
            lock: Mutex::new(()),
            pci_devices: devices,
            current_handle: PciHandle::NoHandle,
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
                    _ => println!("pcid: unknown command: {}", cmd),
                }

                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        let _guard = self.lock.lock();
        if let Some((device, bar_n)) = path.split_once(":") {
            if let Ok(bar_n) = bar_n.parse::<usize>()
                && bar_n < MAX_BARS
            {
                let device = self.find_device(device);
                if let Some(bar) = device.bars[bar_n] {
                    self.current_handle = PciHandle::GetBar(bar);
                }
            } else {
                println!("pcid: invalid bar number");
            }
        }
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();
        match &self.current_handle {
            PciHandle::GetBar(bar) => {
                let (addr, size) = bar.unwrap_mem();
                rstd::mm::physmap(addr, addr, size);
                if offset + buf.len() >= size {
                    return Err(());
                }
                let data =
                    unsafe { core::slice::from_raw_parts((addr + offset) as *const u8, buf.len()) };
                buf.copy_from_slice(data);
            }
            _ => return Err(()),
        }

        Ok(buf.len())
    }

    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();
        match &self.current_handle {
            PciHandle::GetBar(bar) => {
                let (addr, size) = bar.unwrap_mem();
                rstd::mm::physmap(addr, addr, size);
                if offset + buf.len() >= size {
                    return Err(());
                }
                let data = unsafe {
                    core::slice::from_raw_parts_mut((addr + offset) as *mut u8, buf.len())
                };
                data.copy_from_slice(buf);
            }
            _ => return Err(()),
        }

        Ok(buf.len())
    }

    fn size(&mut self) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        match &self.current_handle {
            PciHandle::GetBar(bar) => {
                let (_addr, size) = bar.unwrap_mem();
                return Ok(size);
            }
            _ => return Err(()),
        }
    }

    #[allow(unused_variables)]
    fn ioctl(&mut self, buf: &[usize]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        let cmd = buf[0];
        let arg = buf[1];

        if cmd == 1 {
            match &self.current_handle {
                PciHandle::GetBar(bar) => {
                    let (addr, _size) = bar.unwrap_mem();
                    return Ok(addr);
                }
                _ => return Err(()),
            }
        }

        Err(())
    }

    fn find_device(&self, device: &str) -> &PciDevice {
        self.pci_devices
            .iter()
            .find(|d| match device {
                "ahci" => d.device_type == DeviceType::SataController,
                "nvme" => d.device_type == DeviceType::NvmeController,
                "xhci" => {
                    d.device_type == DeviceType::UsbController && d.device_id.interface == 0x30
                }
                _ => false,
            })
            .unwrap_or_else(|| {
                println!("pcid: device not found: {}", device);
                unsafe { core::hint::unreachable_unchecked() }
            })
    }
}
