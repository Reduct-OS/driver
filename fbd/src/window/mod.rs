use rstd::alloc::{string::String, sync::Arc};
use spin::RwLock;

use crate::fb::FrameBuffer;

use noto_sans_mono_bitmap::{FontWeight, RasterHeight};
use noto_sans_mono_bitmap::{get_raster, get_raster_width};

const FONT_WIDTH: usize = get_raster_width(FontWeight::Regular, FONT_HEIGHT);
const FONT_HEIGHT: RasterHeight = RasterHeight::Size20;

pub struct Window {
    frame_buffer: Arc<RwLock<dyn FrameBuffer>>,
    title: String,
    offset_x: usize,
    offset_y: usize,
    width: usize,
    height: usize,
}

impl Window {
    pub const BACKGROUND_COLOR: u32 = 0x00000000;
    pub const TITLE_BASE_OFFSET_X: usize = 5;
    pub const TITLE_BASE_OFFSET_Y: usize = 5;

    pub fn new(fb: Arc<RwLock<dyn FrameBuffer>>) -> Window {
        Window {
            frame_buffer: fb,
            title: String::new(),
            offset_x: 0,
            offset_y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn set_title<S>(&mut self, title: S) -> &mut Self
    where
        String: From<S>,
    {
        self.title = String::from(title);
        self
    }

    pub fn set_size(&mut self, width: usize, height: usize) -> &mut Self {
        self.width = width;
        self.height = height;
        self
    }

    fn draw_at_buffer(&mut self, buffer: &mut [u32]) {
        buffer.fill(Self::BACKGROUND_COLOR);

        // Draw title
        for (char_i, char) in self.title.chars().enumerate() {
            let char_raster =
                get_raster(char, FontWeight::Regular, FONT_HEIGHT).expect("fbd: unknown char");
            let line_offset = Self::TITLE_BASE_OFFSET_Y;
            for (row_i, row) in char_raster.raster().iter().enumerate() {
                for (col_i, intensity) in row.iter().enumerate() {
                    let (r, g, b) = (*intensity as u32, *intensity as u32, *intensity as u32);
                    // let (r, g, b) = (255 - r, 255 - g, 255 - b);
                    let rgb_32 = /*0 << 24 | */r << 16 | g << 8 | b;

                    let index = char_i * char_raster.width()
                        + col_i
                        + (line_offset + row_i) * self.width
                        + Self::TITLE_BASE_OFFSET_X;

                    buffer[index] = rgb_32;
                }
            }
        }

        let window_body_start_line_y = Self::TITLE_BASE_OFFSET_Y * 2 + FONT_HEIGHT.val() + 1;
        for x in 0..self.width {
            buffer[window_body_start_line_y * self.width + x] = 0x00808080;
        }
    }

    pub fn draw(&mut self) {
        let mut buffer = rstd::alloc::vec![0u32; self.width * self.height];

        self.draw_at_buffer(&mut buffer);

        for x in 0..self.width {
            for y in 0..self.height {
                let dx = self.offset_x + x;
                let dy = self.offset_y + y;
                self.frame_buffer
                    .write()
                    .write(dx, dy, buffer[y * self.width + x]);
            }
        }

        self.frame_buffer.read().flush_buf();
    }
}
