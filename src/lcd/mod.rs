pub use self::color::Color;
pub use self::init::init;
pub use self::stdout::init as init_stdout;

use core::{fmt, ptr};
use stm32f7::stm32f7x6::LTDC;

#[macro_use]
pub mod stdout;
mod color;
mod init;

pub const HEIGHT: usize = 272;
pub const WIDTH: usize = 480;

pub const LAYER_1_OCTETS_PER_PIXEL: usize = 4;
pub const LAYER_1_LENGTH: usize = HEIGHT * WIDTH * LAYER_1_OCTETS_PER_PIXEL;
pub const LAYER_2_OCTETS_PER_PIXEL: usize = 2;
pub const LAYER_2_LENGTH: usize = HEIGHT * WIDTH * LAYER_2_OCTETS_PER_PIXEL;

pub const SDRAM_START: usize = 0xC000_0000;
pub const LAYER_1_START: usize = SDRAM_START;
pub const LAYER_2_START: usize = SDRAM_START + LAYER_1_LENGTH;

pub struct Lcd<'a> {
    controller: &'a mut LTDC,
    layer_1_in_use: bool,
    layer_2_in_use: bool,
}

impl<'a> Lcd<'a> {
    fn new(ltdc: &'a mut LTDC) -> Self {
        Self {
            controller: ltdc,
            layer_1_in_use: false,
            layer_2_in_use: false,
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.controller
            .bccr
            .modify(|_, w| unsafe { w.bc().bits(color.to_rgb()) });
    }

    pub fn layer_1(&mut self) -> Option<Layer<FramebufferArgb8888>> {
        if self.layer_1_in_use {
            None
        } else {
            Some(Layer {
                framebuffer: FramebufferArgb8888::new(LAYER_1_START),
            })
        }
    }

    pub fn layer_2(&mut self) -> Option<Layer<FramebufferAl88>> {
        if self.layer_2_in_use {
            None
        } else {
            Some(Layer {
                framebuffer: FramebufferAl88::new(LAYER_2_START),
            })
        }
    }
}

pub trait Framebuffer {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color);
}

pub struct FramebufferArgb8888 {
    base_addr: usize,
}

impl FramebufferArgb8888 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

impl Framebuffer for FramebufferArgb8888 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * WIDTH + x;
        let pixel_ptr = (self.base_addr + pixel * LAYER_1_OCTETS_PER_PIXEL) as *mut u32;
        unsafe { ptr::write_volatile(pixel_ptr, color.to_argb8888()) };
    }
}

pub struct FramebufferAl88 {
    base_addr: usize,
}

impl FramebufferAl88 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }
}

impl Framebuffer for FramebufferAl88 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * WIDTH + x;
        let pixel_ptr = (self.base_addr + pixel * LAYER_2_OCTETS_PER_PIXEL) as *mut u16;
        unsafe { ptr::write_volatile(pixel_ptr, (color.alpha as u16) << 8 | 0xff) };
    }
}

pub struct Layer<T> {
    framebuffer: T,
}

impl<T: Framebuffer> Layer<T> {
    pub fn horizontal_stripes(&mut self) {
        let colors = [
            0xffffff, 0xcccccc, 0x999999, 0x666666, 0x333333, 0x0, 0xff0000, 0x0000ff,
        ];

        // horizontal stripes
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                self.framebuffer.set_pixel(
                    j,
                    i,
                    Color::from_rgb888(colors[(i / 10) % colors.len()]),
                );
            }
        }
    }

    pub fn vertical_stripes(&mut self) {
        let colors = [
            0xcccccc, 0x999999, 0x666666, 0x333333, 0x0, 0xff0000, 0x0000ff, 0xffffff,
        ];

        // vertical stripes
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                self.framebuffer.set_pixel(
                    j,
                    i,
                    Color::from_rgb888(colors[(j / 10) % colors.len()]),
                );
            }
        }
    }

    pub fn clear(&mut self) {
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                self.framebuffer.set_pixel(j, i, Color::from_argb8888(0));
            }
        }
    }

    pub fn print_point_at(&mut self, x: usize, y: usize) {
        self.print_point_color_at(x, y, Color::from_hex(0xffffff));
    }

    pub fn print_point_color_at(&mut self, x: usize, y: usize, color: Color) {
        assert!(x < WIDTH);
        assert!(y < HEIGHT);

        self.framebuffer.set_pixel(x, y, color);
    }

    pub fn audio_writer(&mut self) -> AudioWriter<T> {
        AudioWriter {
            layer: self,
            next_pixel: 0,
            next_col: 0,
            prev_value: (0, 0),
        }
    }

    pub fn text_writer(&mut self) -> TextWriter<T> {
        TextWriter {
            layer: self,
            x_pos: 0,
            y_pos: 0,
        }
    }
}

pub struct AudioWriter<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    next_pixel: usize,
    next_col: usize,
    prev_value: (usize, usize),
}

impl<'a, T: Framebuffer + 'a> AudioWriter<'a, T> {
    pub fn set_next_pixel(&mut self, color: Color) {
        self.layer
            .print_point_color_at(self.next_pixel % WIDTH, self.next_pixel / WIDTH, color);
        self.next_pixel = (self.next_pixel + 1) % (HEIGHT * WIDTH);
    }

    pub fn layer(&mut self) -> &mut Layer<T> {
        &mut self.layer
    }

    pub fn set_next_col(&mut self, value0: usize, value1: usize) {
        let value0 = value0 + 2usize.pow(15);
        let value0 = value0 as u16 as usize;
        let value0 = value0 / 241;

        let value1 = value1 + 2usize.pow(15);
        let value1 = value1 as u16 as usize;
        let value1 = value1 / 241;

        for i in 0..HEIGHT {
            let mut color = Color::from_argb8888(0);

            if value0 >= self.prev_value.0 {
                if i >= self.prev_value.0 && i <= value0 {
                    color.red = 0xff;
                    color.alpha = 0xff;
                }
            } else if i <= self.prev_value.0 && i >= value0 {
                color.red = 0xff;
                color.alpha = 0xff;
            }

            if value1 >= self.prev_value.1 {
                if i >= self.prev_value.0 && i <= value1 {
                    color.green = 0xff;
                    color.alpha = 0xff;
                }
            } else if i <= self.prev_value.0 && i >= value1 {
                color.green = 0xff;
                color.alpha = 0xff;
            }

            let i = i as usize;
            self.layer.print_point_color_at(self.next_col, i, color);
        }

        self.next_col = (self.next_col + 1) % WIDTH;
        self.prev_value = (value0, value1);
    }
}

pub struct TextWriter<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    x_pos: usize,
    y_pos: usize,
}

impl<'a, T: Framebuffer> TextWriter<'a, T> {
    fn newline(&mut self) {
        self.y_pos += 8;
        self.x_pos = 0;
        if self.y_pos >= HEIGHT {
            self.y_pos = 0;
            self.layer.clear();
        }
    }
}

impl<'a, T: Framebuffer> fmt::Write for TextWriter<'a, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use font8x8::{self, Utf16Fonts};

        for c in s.chars() {
            if c == '\n' {
                self.newline();
                continue;
            }
            let c = c as u16;
            match c {
                0..=0x7f => {
                    let rendered = font8x8::BASIC_FONTS
                        .get(c)
                        .expect("character not found in basic font");
                    for (y, byte) in rendered.iter().enumerate() {
                        for (x, bit) in (0..8).enumerate() {
                            let alpha = if *byte & (1 << bit) == 0 { 0 } else { 255 };
                            let color = Color {
                                red: 255,
                                green: 255,
                                blue: 255,
                                alpha,
                            };
                            self.layer
                                .print_point_color_at(self.x_pos + x, self.y_pos + y, color);
                        }
                    }
                }
                _ => panic!("unprintable character"),
            }
            self.x_pos += 8;
            if self.x_pos >= WIDTH {
                self.newline();
            }
        }
        Ok(())
    }
}
