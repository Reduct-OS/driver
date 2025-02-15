use rstd::{
    fs::{USER_IOCTL, USER_OPEN, USER_READ, USER_SIZE, USER_WRITE, UserCommand},
    println,
};
use spin::Mutex;

use crate::{keyboard::SCANCODE, mouse::MOUSE_CODE};

enum Ps2Handle {
    NoHandle,
    Keyboard,
    Mouse,
}

pub struct Ps2FS {
    lock: Mutex<()>,
    current_handle: Ps2Handle,
    user_command: UserCommand,
}

impl Ps2FS {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            current_handle: Ps2Handle::NoHandle,
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
                    _ => println!("Ps2d: unknown command: {}", cmd),
                }

                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        let _guard = self.lock.lock();

        self.current_handle = match path {
            "keyboard" => Ps2Handle::Keyboard,
            "mouse" => Ps2Handle::Mouse,
            _ => Ps2Handle::NoHandle,
        }
    }

    fn read(&mut self, _offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        Ok(buf.len())
    }

    fn write(&mut self, _offset: usize, buf: &[u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        match self.current_handle {
            Ps2Handle::Keyboard => SCANCODE.lock().push(buf[0]),
            Ps2Handle::Mouse => MOUSE_CODE.lock().push(buf[0]),
            Ps2Handle::NoHandle => return Err(()),
        }

        Ok(1)
    }

    fn size(&mut self) -> Result<usize, ()> {
        let _guard = self.lock.lock();

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
