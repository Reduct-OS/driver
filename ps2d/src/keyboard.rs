use rstd::alloc::vec::Vec;
use spin::Mutex;

pub static SCANCODE: Mutex<Vec<u8>> = Mutex::new(Vec::new());

pub fn init() {}
