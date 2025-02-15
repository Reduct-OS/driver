use rstd::{
    alloc::{string::String, sync::Arc, vec::Vec},
    fs::{FileInfo, InodeTy},
};
use spin::{Lazy, Mutex, RwLock};

use rstd::ref_to_mut;

pub type InodeRef = Arc<RwLock<dyn Inode>>;

pub trait Inode: Sync + Send {
    fn when_mounted(&mut self, path: String, father: Option<InodeRef>);
    fn when_umounted(&mut self);

    fn get_path(&self) -> String;
    fn size(&self) -> usize {
        0
    }

    fn mount(&self, _node: InodeRef, _name: String) {
        unimplemented!()
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> usize {
        0
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> usize {
        0
    }
    fn flush(&self) {
        unimplemented!()
    }

    fn open(&self, _name: String) -> Option<InodeRef> {
        unimplemented!()
    }
    fn create(&self, _name: String, _ty: InodeTy) -> Option<InodeRef> {
        unimplemented!()
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> usize {
        unimplemented!()
    }
    fn list(&self) -> Vec<FileInfo> {
        Vec::new()
    }

    fn inode_type(&self) -> InodeTy {
        InodeTy::File
    }
}

pub struct NullFS {
    path: String,
}

impl NullFS {
    pub fn new() -> InodeRef {
        let inode = Arc::new(RwLock::new(Self {
            path: String::new(),
        }));
        inode
    }
}

impl Inode for NullFS {
    fn when_mounted(&mut self, path: String, _father: Option<InodeRef>) {
        self.path.clear();
        self.path.push_str(path.as_str());
    }

    fn when_umounted(&mut self) {}

    fn get_path(&self) -> String {
        self.path.clone()
    }
}

pub static ROOT: Lazy<Mutex<InodeRef>> = Lazy::new(|| Mutex::new(NullFS::new()));

pub fn user_open(path: String) -> Option<InodeRef> {
    let root = ROOT.lock().clone();

    let path = path.split("/");

    let node = root;

    for path_node in path {
        if path_node.len() > 0 {
            if let Some(child) = node.read().open(String::from(path_node)) {
                core::mem::drop(core::mem::replace(ref_to_mut(&node), child));
            } else {
                return None;
            }
        }
    }

    Some(node.clone())
}
