use rstd::alloc::sync::Arc;
use spin::RwLock;

pub trait FrameBuffer {
    fn write(&mut self, x: usize, y: usize, color: u32);
    fn flush_buf(&self);
}

pub struct Driver {
    pub fb_fd: usize,
    pub width: usize,
    pub height: usize,
    buffer: &'static mut [u32],
}

impl Driver {
    pub fn new() -> Arc<RwLock<Driver>> {
        let mut fd = usize::MAX;
        while fd == usize::MAX {
            fd = rstd::fs::open("/dev/kernel.fb", 2) as usize;

            rstd::proc::r#yield();
        }

        let width = rstd::fs::ioctl(fd, 1, 0) as usize;
        let height = rstd::fs::ioctl(fd, 2, 0) as usize;

        let buffer = rstd::alloc::vec![0u32; width * height].leak();

        let driver = Arc::new(RwLock::new(Driver {
            fb_fd: fd,
            width,
            height,
            buffer,
        }));

        driver.write().init();

        driver
    }

    fn init(&mut self) {
        self.buffer.fill(0x001685A9);
        self.flush();
    }

    pub fn flush(&self) {
        rstd::fs::write(
            self.fb_fd,
            self.buffer.as_ptr() as usize,
            self.buffer.len() * size_of::<u32>(),
        );
    }
}

impl FrameBuffer for Driver {
    fn write(&mut self, x: usize, y: usize, color: u32) {
        self.buffer[y * self.width + x] = color;
    }

    fn flush_buf(&self) {
        self.flush();
    }
}
