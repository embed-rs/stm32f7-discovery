#![allow(dead_code)]

pub use self::color::Color;
pub use self::init::init;
pub use self::stdout::init as init_stdout;

use board::ltdc::Ltdc;
use embedded::interfaces::gpio::OutputPin;
use core::{fmt, ptr};
use self::font::FontRenderer;

#[macro_use]
pub mod stdout;
mod init;
mod color;
mod font;

const SDRAM_START: usize = 0xC000_0000;
const LAYER_1_START: usize = SDRAM_START;
const LAYER_2_START: usize = SDRAM_START + 272 * 480 * 4;

static TTF: &[u8] = include_bytes!("../../RobotoMono-Bold.ttf");

pub struct Lcd {
    controller: &'static mut Ltdc,
    display_enable: OutputPin,
    backlight_enable: OutputPin,
    layer_1_in_use: bool,
    layer_2_in_use: bool,
}

impl Lcd {
    pub fn set_background_color(&mut self, color: Color) {
        self.controller.bccr.update(|r| r.set_bc(color.to_rgb()));
    }

    pub fn layer_1(&mut self) -> Option<Layer<FramebufferArgb8888>> {
        if self.layer_1_in_use {
            None
        } else {
            Some(Layer { framebuffer: FramebufferArgb8888::new(LAYER_1_START) })
        }
    }

    pub fn layer_2(&mut self) -> Option<Layer<FramebufferAl88>> {
        if self.layer_2_in_use {
            None
        } else {
            Some(Layer { framebuffer: FramebufferAl88::new(LAYER_2_START) })
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
        let pixel = y * 480 + x;
        let pixel_ptr = (self.base_addr + pixel * 4) as *mut u32;
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
        let pixel = y * 480 + x;
        let pixel_ptr = (self.base_addr + pixel * 2) as *mut u16;
        unsafe { ptr::write_volatile(pixel_ptr, (color.alpha as u16) << 8 | 0xff) };
    }
}

pub struct Layer<T> {
    framebuffer: T,
}

impl<T: Framebuffer> Layer<T> {
    pub fn horizontal_stripes(&mut self) {
        let colors = [0xffffff, 0xcccccc, 0x999999, 0x666666, 0x333333, 0x0, 0xff0000, 0x0000ff];

        // horizontal stripes
        for i in 0..272 {
            for j in 0..480 {
                self.framebuffer
                    .set_pixel(j, i, Color::from_rgb888(colors[(i / 10) % colors.len()]));
            }
        }
    }

    pub fn vertical_stripes(&mut self) {
        let colors = [0xcccccc, 0x999999, 0x666666, 0x333333, 0x0, 0xff0000, 0x0000ff, 0xffffff];

        // vertical stripes
        for i in 0..272 {
            for j in 0..480 {
                self.framebuffer
                    .set_pixel(j, i, Color::from_rgb888(colors[(j / 10) % colors.len()]));
            }
        }
    }

    pub fn clear(&mut self) {
        for i in 0..272 {
            for j in 0..480 {
                self.framebuffer.set_pixel(j, i, Color::from_argb8888(0));
            }
        }
    }

    pub fn print_point_at(&mut self, x: usize, y: usize) {
        self.print_point_color_at(x, y, Color::from_hex(0xffffff));
    }

    pub fn print_point_color_at(&mut self, x: usize, y: usize, color: Color) {
        assert!(x < 480);
        assert!(y < 272);

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
            font_renderer: FontRenderer::new(TTF, 14.0),
            x_pos: 0,
            y_pos: 0,
        }
    }
}

pub struct AudioWriter<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    next_pixel: usize,
    next_col: usize,
    prev_value: (u32, u32),
}

impl<'a, T: Framebuffer + 'a> AudioWriter<'a, T> {
    pub fn set_next_pixel(&mut self, color: Color) {
        self.layer
            .print_point_color_at(self.next_pixel % 480, self.next_pixel / 480, color);
        self.next_pixel = (self.next_pixel + 1) % (272 * 480);
    }

    pub fn layer(&mut self) -> &mut Layer<T> {
        &mut self.layer
    }

    pub fn set_next_col(&mut self, value0: u32, value1: u32) {
        let value0 = value0 + 2u32.pow(15);
        let value0 = value0 as u16 as u32;
        let value0 = value0 / 241;

        let value1 = value1 + 2u32.pow(15);
        let value1 = value1 as u16 as u32;
        let value1 = value1 / 241;

        for i in 0..272 {
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


        self.next_col = (self.next_col + 1) % 480;
        self.prev_value = (value0, value1);
    }
}

pub struct TextWriter<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    font_renderer: FontRenderer<'a>,
    x_pos: usize,
    y_pos: usize,
}

impl <'a, T: Framebuffer> TextWriter<'a, T> {
    fn write_str_no_newlines(&mut self, s: &str) -> fmt::Result {
        let font_height = self.font_renderer.font_height() as usize;
        let &mut TextWriter {
                     ref mut layer,
                     ref mut font_renderer,
                     ref mut x_pos,
                     ref mut y_pos,
                     ..
                 } = self;

        let width = font_renderer.render(s, |x, y, v| {
            if *x_pos + x >= 480 {
                *x_pos = 0;
                *y_pos += font_height;
            }
            let alpha = (v * 255.0 + 0.5) as u8;
            let color = Color {
                red: 255,
                green: 255,
                blue: 255,
                alpha,
            };
            layer.print_point_color_at(*x_pos + x, *y_pos + y, color);
        });
        *x_pos += width;
        Ok(())
    }
}

impl<'a, T: Framebuffer> fmt::Write for TextWriter<'a, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut lines = s.split('\n').peekable();
        while let Some(line) = lines.next() {
            self.write_str_no_newlines(line)?;
            if lines.peek().is_some() {
                self.x_pos = 0;
                self.y_pos += self.font_renderer.font_height() as usize;
            }
        }
        Ok(())
    }
}
