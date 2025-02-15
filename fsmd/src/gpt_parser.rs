use gpt_disk_io::{
    BlockIo, Disk, DiskError,
    gpt_disk_types::{BlockSize, GptPartitionEntryArrayLayout, GptPartitionEntrySize, Lba},
};
use rstd::alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Mutex, RwLock};

use crate::inode::{Inode, InodeRef};

pub struct PartitionInode {
    offset: usize,
    size: usize,
    drive: InodeRef,
    path: String,
}

impl PartitionInode {
    pub fn new(offset: usize, size: usize, drive: InodeRef) -> InodeRef {
        Arc::new(RwLock::new(Self {
            offset,
            size,
            drive,
            path: String::new(),
        }))
    }
}

impl Inode for PartitionInode {
    fn when_mounted(&mut self, path: String, _father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn size(&self) -> usize {
        self.size
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let offset = self.offset + offset;
        self.drive.read().read_at(offset, buf)
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let offset = self.offset + offset;
        self.drive.read().write_at(offset, buf)
    }
}

struct InodeRefIO {
    inode: InodeRef,
}

impl InodeRefIO {
    pub fn new(inode: InodeRef) -> Self {
        Self { inode }
    }
}

impl BlockIo for InodeRefIO {
    type Error = usize;

    fn block_size(&self) -> gpt_disk_io::gpt_disk_types::BlockSize {
        BlockSize::from_usize(512).unwrap()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn num_blocks(&mut self) -> Result<u64, Self::Error> {
        Ok((self.inode.read().size() / 512) as u64)
    }

    fn read_blocks(
        &mut self,
        start_lba: gpt_disk_io::gpt_disk_types::Lba,
        dst: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.inode.read().read_at(start_lba.0 as usize * 512, dst);
        Ok(())
    }

    fn write_blocks(
        &mut self,
        start_lba: gpt_disk_io::gpt_disk_types::Lba,
        src: &[u8],
    ) -> Result<(), Self::Error> {
        self.inode.read().write_at(start_lba.0 as usize * 512, src);
        Ok(())
    }
}

pub static PARTITIONS: Mutex<Vec<InodeRef>> = Mutex::new(Vec::new());

pub fn parse_gpt_disk(disk: InodeRef) -> Result<(), DiskError<usize>> {
    let io = InodeRefIO::new(disk.clone());
    let mut gpt = Disk::new(io)?;

    let mut buf = Vec::new();
    for _ in 0..512 * 8 * 100 {
        buf.push(0);
    }

    let header = gpt.read_gpt_header(Lba(1), &mut buf)?;

    let mut buf = Vec::new();
    for _ in 0..512 * 8 * 100 {
        buf.push(0);
    }

    let part_iter = gpt.gpt_partition_entry_array_iter(
        GptPartitionEntryArrayLayout {
            start_lba: header.partition_entry_lba.into(),
            entry_size: GptPartitionEntrySize::new(header.size_of_partition_entry.to_u32())
                .ok()
                .ok_or(DiskError::Io(0))?,
            num_entries: header.number_of_partition_entries.to_u32(),
        },
        &mut buf,
    )?;

    for part in part_iter {
        if let Ok(part) = part {
            if !part.is_used() {
                break;
            }
            let start_offset = part.starting_lba.to_u64() as usize * 512;
            let size = part.ending_lba.to_u64() as usize * 512;

            let partition = PartitionInode::new(start_offset, size, disk.clone());

            PARTITIONS.lock().push(partition);
        }
    }

    drop(buf);

    Ok(())
}
