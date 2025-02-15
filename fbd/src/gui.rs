use rstd::{println, ref_to_mut};
use spin::Mutex;

pub struct Gui {
    lock: Mutex<()>,
    buffer: &'static mut [u32],
    fb_fd: usize,
    width: usize,
    height: usize,
}

impl Gui {
    pub fn new() -> Gui {
        let fb_fd = rstd::fs::open("/dev/kernel.fb", 1) as usize;

        let width = rstd::fs::ioctl(fb_fd, 1, 0) as usize;
        let height = rstd::fs::ioctl(fb_fd, 2, 0) as usize;

        println!("fbd: width = {}, height = {}", width, height);

        let buffer = rstd::alloc::vec![0u32; width * height].leak();
        assert_eq!(buffer.len(), width * height);

        Gui {
            lock: Mutex::new(()),
            buffer,
            width,
            height,
            fb_fd,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn draw_background(&mut self) {
        let _guard = self.lock.lock();

        self.buffer.fill(0x001685A9);
    }

    pub fn main_loop(&mut self) -> ! {
        self.draw_background();

        loop {
            self.flush();

            rstd::proc::r#yield();
        }
    }

    pub fn flush(&self) {
        let _guard = self.lock.lock();

        rstd::fs::write(
            self.fb_fd,
            ref_to_mut(self).buffer.as_mut_ptr() as usize,
            self.buffer.len() * size_of::<u32>(),
        );
    }
}
