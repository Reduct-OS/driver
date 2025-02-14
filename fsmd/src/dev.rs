use rstd::{
    alloc::{string::String, sync::Arc},
    fs::{Inode, InodeRef},
};
use spin::RwLock;

pub struct DevInode {
    path: String,
    inner: usize,
}

impl DevInode {
    pub fn new(fd: usize) -> InodeRef {
        Arc::new(RwLock::new(Self {
            path: String::new(),
            inner: fd,
        }))
    }
}

impl Inode for DevInode {
    fn when_mounted(&mut self, path: String, _father: Option<rstd::fs::InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        rstd::fs::lseek(self.inner, offset);
        let len = rstd::fs::read(self.inner, buf.as_mut_ptr() as usize, buf.len());
        rstd::fs::lseek(self.inner, 0);
        return len as usize;
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        rstd::fs::lseek(self.inner, offset);
        let len = rstd::fs::write(self.inner, buf.as_ptr() as usize, buf.len());
        rstd::fs::lseek(self.inner, 0);
        return len as usize;
    }

    fn size(&self) -> usize {
        let mut stat = rstd::stat::Stat::default();
        rstd::fs::fstat(self.inner, stat.as_mut_ptr() as usize);
        let dev_fsize = stat.st_size;
        return dev_fsize as usize;
    }
}
