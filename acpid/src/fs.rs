use rstd::{
    alloc::{string::String, vec::Vec},
    fs::{USER_LIST, USER_OPEN, USER_READ, USER_SIZE, UserCommand},
};

use crate::acpi::{AcpiContext, SdtSignature};

fn parse_hex_digit(hex: u8) -> Option<u8> {
    let hex = hex.to_ascii_lowercase();

    if hex >= b'a' && hex <= b'f' {
        Some(hex - b'a' + 10)
    } else if hex >= b'0' && hex <= b'9' {
        Some(hex - b'0')
    } else {
        None
    }
}

fn parse_hex_2digit(hex: &[u8]) -> Option<u8> {
    parse_hex_digit(hex[0])
        .and_then(|most_significant| Some((most_significant << 4) | parse_hex_digit(hex[1])?))
}

fn parse_oem_id(hex: [u8; 12]) -> Option<[u8; 6]> {
    Some([
        parse_hex_2digit(&hex[0..2])?,
        parse_hex_2digit(&hex[2..4])?,
        parse_hex_2digit(&hex[4..6])?,
        parse_hex_2digit(&hex[6..8])?,
        parse_hex_2digit(&hex[8..10])?,
        parse_hex_2digit(&hex[10..12])?,
    ])
}
fn parse_oem_table_id(hex: [u8; 16]) -> Option<[u8; 8]> {
    Some([
        parse_hex_2digit(&hex[0..2])?,
        parse_hex_2digit(&hex[2..4])?,
        parse_hex_2digit(&hex[4..6])?,
        parse_hex_2digit(&hex[6..8])?,
        parse_hex_2digit(&hex[8..10])?,
        parse_hex_2digit(&hex[10..12])?,
        parse_hex_2digit(&hex[12..14])?,
        parse_hex_2digit(&hex[14..16])?,
    ])
}

fn parse_table(table: &[u8]) -> Option<SdtSignature> {
    let signature_part = table.get(..4)?;
    let first_hyphen = table.get(4)?;
    let oem_id_part = table.get(5..17)?;
    let second_hyphen = table.get(17)?;
    let oem_table_part = table.get(18..34)?;

    if *first_hyphen != b'-' {
        return None;
    }
    if *second_hyphen != b'-' {
        return None;
    }

    if table.len() > 34 {
        return None;
    }

    Some(SdtSignature {
        signature: <[u8; 4]>::try_from(signature_part)
            .expect("expected 4-byte slice to be convertible into [u8; 4]"),
        oem_id: {
            let hex = <[u8; 12]>::try_from(oem_id_part)
                .expect("expected 12-byte slice to be convertible into [u8; 12]");
            parse_oem_id(hex)?
        },
        oem_table_id: {
            let hex = <[u8; 16]>::try_from(oem_table_part)
                .expect("expected 16-byte slice to be convertible into [u8; 16]");
            parse_oem_table_id(hex)?
        },
    })
}

enum AcpiHandle {
    NoHandle,
    TopLevel,
    Tables,
    Table(SdtSignature),
}

impl AcpiHandle {
    fn len(&self, acpi_ctx: &AcpiContext) -> Result<usize, ()> {
        Ok(match self {
            // Files
            Self::Table(signature) => acpi_ctx.sdt_from_signature(signature).ok_or(())?.length(),
            // Directories
            Self::TopLevel | Self::NoHandle | Self::Tables => 0,
        })
    }
}

pub struct AcpiFS {
    acpi_context: AcpiContext,
    current_handle: AcpiHandle,
    user_command: UserCommand,
}

impl AcpiFS {
    pub fn new(ctx: AcpiContext) -> Self {
        Self {
            acpi_context: ctx,
            current_handle: AcpiHandle::NoHandle,
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
                    USER_READ => {
                        if self
                            .read(unsafe {
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
                    _ => println!("acpid: unknown command: {}", cmd),
                }
                self.user_command.ok_signal = usize::MAX;
                self.user_command.cmd = 0;
            }

            rstd::proc::r#yield();
        }
    }

    fn open(&mut self, path: &str) {
        match path {
            "" => {
                self.current_handle = AcpiHandle::TopLevel;
            }
            "tables" => {
                self.current_handle = AcpiHandle::Tables;
            }
            _ => self.open_table(path),
        }
    }

    fn open_table(&mut self, path: &str) {
        if let Some((tables, table)) = path.split_once(':') {
            match tables {
                "tables" => {
                    if let Some(table) = parse_table(table.as_bytes()) {
                        self.current_handle = AcpiHandle::Table(table);
                    }
                }
                _ => println!("Unknown path: {}", tables),
            }
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let src_buf = match &self.current_handle {
            AcpiHandle::Table(signature) => self
                .acpi_context
                .sdt_from_signature(signature)
                .ok_or(())?
                .as_slice(),
            _ => return Err(()),
        };

        let offset = core::cmp::min(src_buf.len(), self.user_command.offset);
        let src_buf = &src_buf[offset..];

        let to_copy = core::cmp::min(src_buf.len(), buf.len());

        buf[..to_copy].copy_from_slice(&src_buf[..to_copy]);

        self.current_handle = AcpiHandle::NoHandle;

        Ok(to_copy)
    }

    fn size(&mut self) -> Result<usize, ()> {
        self.current_handle
            .len(&self.acpi_context)
            .try_into()
            .unwrap_or(Err(()))
    }

    fn list(&mut self) -> Result<(usize, usize, usize), ()> {
        match &self.current_handle {
            AcpiHandle::Tables => {
                let mut result = Vec::new();

                for table in self.acpi_context.tables().iter() {
                    let utf8_or_eio = |bytes| str::from_utf8(bytes).map_err(|_| ());

                    let mut name = String::new();
                    name.push_str(utf8_or_eio(&table.signature[..])?);
                    name.push('-');
                    for byte in table.oem_id.iter() {
                        core::fmt::write(&mut name, format_args!("{:>02X}", byte)).unwrap();
                    }
                    name.push('-');
                    for byte in table.oem_table_id.iter() {
                        core::fmt::write(&mut name, format_args!("{:>02X}", byte)).unwrap();
                    }

                    result.push(name);
                }

                let (ret_struct_addr, ret_struct_len, ret_struct_cap) = result.into_raw_parts();

                Ok((ret_struct_addr as usize, ret_struct_len, ret_struct_cap))
            }
            _ => return Err(()),
        }
    }
}
