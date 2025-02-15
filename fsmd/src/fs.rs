use rstd::{
    alloc::{
        collections::btree_map::BTreeMap,
        string::{String, ToString},
    },
    fs::{USER_LIST, USER_OPEN, USER_READ, USER_SIZE, USER_WRITE, UserCommand},
    println,
};
use spin::Mutex;

use crate::inode::{InodeRef, user_open};

enum FSHandle {
    ErrHandle,
    RwHandle(InodeRef),
}

pub struct FsmFS {
    lock: Mutex<()>,
    handles: BTreeMap<String, FSHandle>,
    user_command: UserCommand,
}

impl FsmFS {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            handles: BTreeMap::new(),
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
                    USER_LIST => {
                        if let Ok((struct_ptr, len, cap)) = self.list() {
                            self.user_command.ret_val = struct_ptr as isize;
                            self.user_command.ret_val2 = len as isize;
                            self.user_command.ret_val3 = cap as isize;
                        } else {
                            self.user_command.ret_val = 0;
                        }
                    }
                    _ => println!("fsmd: unknown command: {}", cmd),
                }

                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        let _guard = self.lock.lock();

        let open_path = path.to_string();

        let inode = user_open(open_path.clone());
        if inode.is_some() {
            self.handles
                .insert(open_path, FSHandle::RwHandle(inode.unwrap()));
            return;
        }
        self.handles.insert(open_path, FSHandle::ErrHandle);
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        let path_addr = self.user_command.ret_val as *const u8;
        let path_len = self.user_command.ret_val2 as usize;
        let str =
            unsafe { str::from_utf8(core::slice::from_raw_parts(path_addr, path_len)).unwrap() };

        let handle = self.handles.get(&str.to_string());
        if let Some(handle) = handle {
            match handle {
                FSHandle::ErrHandle => return Err(()),
                FSHandle::RwHandle(inode) => {
                    inode.read().read_at(offset, buf);
                }
            }
        }

        Ok(buf.len())
    }

    fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        let path_addr = self.user_command.ret_val as *const u8;
        let path_len = self.user_command.ret_val2 as usize;
        let str =
            unsafe { str::from_utf8(core::slice::from_raw_parts(path_addr, path_len)).unwrap() };

        let handle = self.handles.get(&str.to_string());
        if let Some(handle) = handle {
            match handle {
                FSHandle::ErrHandle => return Err(()),
                FSHandle::RwHandle(inode) => {
                    inode.read().write_at(offset, buf);
                }
            }
        }

        Ok(buf.len())
    }

    fn size(&mut self) -> Result<usize, ()> {
        let _guard = self.lock.lock();

        let path_addr = self.user_command.ret_val as *const u8;
        let path_len = self.user_command.ret_val2 as usize;
        let str =
            unsafe { str::from_utf8(core::slice::from_raw_parts(path_addr, path_len)).unwrap() };

        let handle = self.handles.get(&str.to_string());
        if let Some(handle) = handle {
            match handle {
                FSHandle::ErrHandle => return Err(()),
                FSHandle::RwHandle(inode) => {
                    return Ok(inode.read().size());
                }
            }
        }

        Err(())
    }

    fn list(&mut self) -> Result<(usize, usize, usize), ()> {
        let _guard = self.lock.lock();

        let path_addr = self.user_command.ret_val as *const u8;
        let path_len = self.user_command.ret_val2 as usize;
        let str =
            unsafe { str::from_utf8(core::slice::from_raw_parts(path_addr, path_len)).unwrap() };

        let handle = self.handles.get(&str.to_string());
        if let Some(handle) = handle {
            match handle {
                FSHandle::ErrHandle => return Err(()),
                FSHandle::RwHandle(inode) => {
                    let result = inode.read().list();

                    let (ret_struct_addr, ret_struct_len, ret_struct_cap) = result.into_raw_parts();

                    return Ok((ret_struct_addr as usize, ret_struct_len, ret_struct_cap));
                }
            }
        }

        Err(())
    }
}
