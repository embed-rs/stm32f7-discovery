#![allow(dead_code)]

pub use self::color::Color;
pub use self::init::init;

use board::ltdc::Ltdc;
use embedded::interfaces::gpio::OutputPin;
use font_render;
use core::{fmt, ptr, ops};

mod init;
mod color;
mod dma2d;

const SDRAM_START: usize = 0xC000_0000;
const LAYER_1_START: usize = SDRAM_START;
const LAYER_2_START: usize = SDRAM_START + 272*480*4;

pub struct Lcd {
    controller: &'static mut Ltdc,
    dma2d: dma2d::Dma2d<'static>,
    display_enable: OutputPin,
    backlight_enable: OutputPin,
    layer_1: Layer<FramebufferArgb8888>,
    layer_2: Option<Layer<FramebufferArgb4444>>,
}

impl Lcd {
    pub fn set_background_color(&mut self, color: Color) {
        self.controller
            .bccr
            .update(|r| r.set_bc(color.to_rgb()));
    }

    pub fn fill_with_color(&mut self, color: Color) {
        self.dma2d.fill_color(self.layer_1.framebuffer.base_addr, 480, 272, 0, color);
    }

    pub fn fill_rect_with_color(&mut self, rect: Rectangle, color: Color) {
        let addr_offset = (rect.y_0 as usize * 480 + rect.x_0 as usize) * 4;
        let addr = self.layer_1.framebuffer.base_addr + addr_offset;
        let pixel_per_line = rect.x_1 - rect.x_0;
        let number_of_lines = rect.y_1 - rect.y_0;
        let line_offset = 480 - pixel_per_line;
        self.dma2d.fill_color(addr, pixel_per_line, number_of_lines, line_offset, color);
    }

    pub fn copy_alpha_slice_to(&mut self, data: &[u8], rect: Rectangle) {
        let out_addr_offset = (rect.y_0 as usize * 480 + rect.x_0 as usize) * 4;
        let out_addr = self.layer_1.framebuffer.base_addr + out_addr_offset;
        let pixel_per_line = rect.x_1 - rect.x_0;
        let number_of_lines = rect.y_1 - rect.y_0;
        let out_line_offset = 480 - pixel_per_line;

        let fg_addr = data.as_ptr() as usize;
        let fg_line_offset = 0;
        let fg_pfc = dma2d::Pfc::A8;
        let fg_color = Color::from_hex(0xffffff);

        let bg_addr = out_addr;
        let bg_line_offset = out_line_offset;
        let bg_pfc = dma2d::Pfc::Argb8888;

        self.dma2d.memory_to_memory_blending(fg_addr, fg_line_offset, fg_pfc, fg_color,
            bg_addr, bg_line_offset, bg_pfc,
            out_addr, out_line_offset,
            pixel_per_line, number_of_lines);
    }

    pub fn test(&mut self) {
        self.dma2d.test();
    }

    pub fn layer_2(&mut self) -> Option<Layer<FramebufferArgb4444>> {
        self.layer_2.take()
    }
}

impl ops::Deref for Lcd {
    type Target = Layer<FramebufferArgb8888>;

    fn deref(&self) -> &Self::Target {
        &self.layer_1
    }
}

impl ops::DerefMut for Lcd {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.layer_1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rectangle {
    pub x_0: u16,
    pub x_1: u16,
    pub y_0: u16,
    pub y_1: u16,
}

pub trait Framebuffer {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color);
}

pub struct FramebufferArgb8888 {
    base_addr: usize,
}

impl FramebufferArgb8888 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr, }
    }
}

impl Framebuffer for FramebufferArgb8888 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * 480 + x;
        let pixel_ptr = (self.base_addr + pixel * 4) as *mut u32;
        unsafe { ptr::write_volatile(pixel_ptr, color.to_argb8888()) };
    }
}

pub struct FramebufferRgb888 {
    base_addr: usize,
}

impl FramebufferRgb888 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr, }
    }
}

impl Framebuffer for FramebufferRgb888 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * 480 + x;
        let red_ptr = (self.base_addr + pixel * 3) as *mut u8;
        let green_ptr = (self.base_addr + pixel * 3 + 1) as *mut u8;
        let blue_ptr = (self.base_addr + pixel * 3 + 2) as *mut u8;
        unsafe {
            ptr::write_volatile(red_ptr, color.red);
            ptr::write_volatile(green_ptr, color.green);
            ptr::write_volatile(blue_ptr, color.blue);
        };
    }
}

pub struct FramebufferArgb4444 {
    base_addr: usize,
}

impl FramebufferArgb4444 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr, }
    }
}

impl Framebuffer for FramebufferArgb4444 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * 480 + x;
        let pixel_ptr = (self.base_addr + pixel * 2) as *mut u16;
        unsafe { ptr::write_volatile(pixel_ptr, color.to_argb4444()) };
    }
}

pub struct FramebufferAl88 {
    base_addr: usize,
}

impl FramebufferAl88 {
    fn new(base_addr: usize) -> Self {
        Self { base_addr, }
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
                self.framebuffer.set_pixel(j, i, Color::from_rgb888(colors[i / 10]));
            }
        }
    }

    pub fn vertical_stripes(&mut self) {
        let colors = [0xcccccc, 0x999999, 0x666666, 0x333333, 0x0, 0xff0000, 0x0000ff, 0xffffff];

        // vertical stripes
        for i in 0..272 {
            for j in 0..480 {
                self.framebuffer.set_pixel(j, i, Color::from_rgb888(colors[j / 10]));
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

    pub fn text_writer(&mut self) -> Result<TextWriterImpl<T>, font_render::Error> {
        Ok(TextWriterImpl {
            layer: self,
            writer: font_render::TextWriter::default()?,
        })
    }
}

pub trait TextWriter {
    fn print_char(&mut self, c: char);

    fn print_str(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }

    fn set_offset(&mut self, off_x: usize, off_y: usize);

    fn width_height(&self, s: &str) -> (u32, u32);
}

pub struct TextWriterImpl<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    writer: font_render::TextWriter<'a>,
}

impl<'a, T: Framebuffer> TextWriter for TextWriterImpl<'a, T> {
    fn print_char(&mut self, c: char) {
        let &mut TextWriterImpl {ref mut layer, ref mut writer} = self;
        writer.print_char(c, |coords, value| {
            let color = Color::rgba(255, 255, 255, value);
            layer.print_point_color_at(coords.x, coords.y, color);
        });
    }

    fn print_str(&mut self, s: &str) {
        for c in s.chars() {
            self.print_char(c);
        }
    }

    fn set_offset(&mut self, off_x: usize, off_y: usize) {
        self.writer.set_offset(off_x, off_y);
    }

    fn width_height(&self, s: &str) -> (u32, u32) {
        self.writer.width_height(s)
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
        self.layer.print_point_color_at(self.next_pixel % 480, self.next_pixel / 480, color);
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


impl<'a, T: Framebuffer> fmt::Write for TextWriterImpl<'a, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print_str(s);
        Ok(())
    }
}
