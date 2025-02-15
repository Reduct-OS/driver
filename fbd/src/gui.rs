use fur::display::DisplayDriver;
use rstd::alloc::sync::Arc;
use spin::RwLock;

pub struct Driver {
    fb_fd: usize,
    width: usize,
    height: usize,
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

impl DisplayDriver for Driver {
    fn read(
        &self,
        _x: usize,
        _y: usize,
        _width: usize,
        _height: usize,
        _pixels: &mut [fur::color::Color],
    ) {
    }

    fn write(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: &fur::color::Color,
    ) {
        for dx in 0..width {
            for dy in 0..height {
                let t_x = dx + x;
                let t_y = dy + y;
                self.buffer[t_y * self.width + t_x] = color.as_0rgb_u32();
            }
        }

        self.flush();
    }

    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}
