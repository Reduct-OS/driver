use core::ops::Deref;

use crate::acpi::AcpiContext;

#[derive(Default)]
pub struct UserCommand {
    pub cmd: usize,
    pub offset: usize,
    pub buf_addr: usize,
    pub buf_size: usize,
    pub ok_signal: usize,
}

impl UserCommand {
    pub fn new(cmd: usize, offset: usize, buf_addr: usize, buf_size: usize) -> UserCommand {
        Self {
            cmd,
            offset,
            buf_addr,
            buf_size,
            ok_signal: 0,
        }
    }
}

impl Deref for UserCommand {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self as *const UserCommand as *const u8, size_of::<Self>())
        }
    }
}

pub struct AcpiFS {
    acpi_context: AcpiContext,
    user_command: UserCommand,
}

impl AcpiFS {
    pub fn new(ctx: AcpiContext) -> Self {
        Self {
            acpi_context: ctx,
            user_command: UserCommand::default(),
        }
    }
}

impl AcpiFS {
    pub fn fs_addr(&mut self) -> usize {
        &mut self.user_command as *mut UserCommand as usize
    }

    pub fn while_parse(&mut self) -> ! {
        loop {
            let cmd = self.user_command.cmd;
            if cmd != 0 {
                println!("Come on some command!!!");
                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }
}
