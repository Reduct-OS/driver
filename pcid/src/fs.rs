use pcid::PciDevice;
use rstd::{
    alloc::vec::Vec,
    fs::{USER_OPEN, UserCommand},
};

pub struct PciFS {
    pci_devices: Vec<PciDevice>,
    current_device: Option<PciDevice>,
    user_command: UserCommand,
}

impl PciFS {
    pub fn new(devices: Vec<PciDevice>) -> Self {
        Self {
            pci_devices: devices,
            current_device: None,
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
                    USER_OPEN => self.open(
                        str::from_utf8(unsafe {
                            core::slice::from_raw_parts(
                                self.user_command.buf_addr as *const u8,
                                self.user_command.buf_size,
                            )
                        })
                        .unwrap(),
                    ),
                    _ => println!("pcid: unknown command: {}", cmd),
                }

                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        println!("pcid: open path: {}", path);
    }
}
