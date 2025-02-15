use rstd::alloc::vec::Vec;
use spin::Mutex;

pub static MOUSE_CODE: Mutex<Vec<u8>> = Mutex::new(Vec::new());

pub fn init() {
    // todo
}
