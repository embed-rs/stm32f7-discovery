#![allow(dead_code)]

pub use self::color::Color;
pub use self::init::init;

use board::ltdc::Ltdc;
use embedded::interfaces::gpio::OutputPin;
use core::ptr;

mod init;
mod color;

// Use the SDRAM as a framebuffer
pub const FRAMEBUFFER_BASE_ADDRESS: u32 = 0xC000_0000;
// It's a 480x272 16-bit LCD
pub const WIDTH: u16 = 480;
pub const HEIGHT: u16 = 272;
pub const NUM_PIXELS: u32 = WIDTH as u32 * HEIGHT as u32;
pub const OCTETS_PER_PIXEL: u32 = 2;
pub const FRAMEBUFFER_LEN: u32 = NUM_PIXELS * OCTETS_PER_PIXEL;

pub struct Lcd {
    controller: &'static mut Ltdc,
    display_enable: OutputPin,
    backlight_enable: OutputPin,
    next_pixel: u32,
    next_col: u16,
    prev_value: (u32, u32),
}

#[derive(Copy, Clone)]
pub enum Buffer {
    Primary,
    Secondary,
}

impl Lcd {
    pub fn set_background_color(&mut self, color: Color) {
        self.controller.bccr.update(|r| r.set_bc(color.to_rgb()));
    }

    fn set_pixel_raw(&mut self, x: u16, y: u16, color: u16, buffer: Buffer) {
        let addr = match buffer {
            Buffer::Primary => FRAMEBUFFER_BASE_ADDRESS,
            Buffer::Secondary => FRAMEBUFFER_BASE_ADDRESS + FRAMEBUFFER_LEN,
        };
        let pixel = x as u32 + (y as u32 * WIDTH as u32);
        let pixel_color = (addr + pixel * OCTETS_PER_PIXEL) as *mut u16;
        unsafe { ptr::write_volatile(pixel_color, color) };
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color, buffer: Buffer) {
        self.set_pixel_raw(x, y, color.to_argb1555(), buffer)
    }

    /// Fills in a region
    /// Region includes the top_left point, but does not include the bottom_right point.
    /// That is (0, 0) and (WIDTH, HEIGHT) means the whole screen.
    pub fn fill_region(&mut self,
                       top_left: (u16, u16),
                       bottom_right: (u16, u16),
                       color: Color,
                       buffer: Buffer) {
        let color = color.to_argb1555();
        for y in top_left.1..bottom_right.1 {
            for x in top_left.0..bottom_right.0 {
                self.set_pixel_raw(x, y, color, buffer);
            }
        }
    }

    pub fn test_pixels(&mut self) {
        self.fill_region((0, 0), (WIDTH / 2, HEIGHT / 2), Color::rgb(255, 255, 255), Buffer::Primary);
        self.fill_region((WIDTH / 2, 0), (WIDTH, HEIGHT / 2), Color::rgb(255, 0, 0), Buffer::Primary);
        self.fill_region((0, HEIGHT / 2), (WIDTH / 2, HEIGHT), Color::rgb(0, 255, 0), Buffer::Primary);
        self.fill_region((WIDTH / 2, HEIGHT / 2), (WIDTH, HEIGHT), Color::rgb(0, 0, 255), Buffer::Primary);
    }

    pub fn clear_screen(&mut self) {
        use core;
        unsafe {
            core::intrinsics::volatile_set_memory(FRAMEBUFFER_BASE_ADDRESS as *mut u8,
                                                  0,
                                                  FRAMEBUFFER_LEN as usize * 2);
        }
    }

    pub fn set_next_pixel(&mut self, color: u16) {
        // layer 1
        let pixel_color = (FRAMEBUFFER_BASE_ADDRESS + self.next_pixel * OCTETS_PER_PIXEL) as
                          *mut u16;
        unsafe { ptr::write_volatile(pixel_color, color) };
        self.next_pixel = (self.next_pixel + 1) % NUM_PIXELS;
    }

    pub fn set_next_col(&mut self, value0: u32, value1: u32) {
        let value0 = value0 + 2u32.pow(15);
        let value0 = value0 as u16 as u32;
        let value0 = value0 / 241;

        let value1 = value1 + 2u32.pow(15);
        let value1 = value1 as u16 as u32;
        let value1 = value1 / 241;

        // layer 1
        for y in 0..HEIGHT as u32 {
            let mut color = 0;

            if value0 >= self.prev_value.0 {
                if y >= self.prev_value.0 && y <= value0 {
                    color |= 0xff00;
                }
            } else if y <= self.prev_value.0 && y >= value0 {
                color |= 0xff00;
            }

            if value1 >= self.prev_value.1 {
                if y >= self.prev_value.0 && y <= value1 {
                    color |= 0x00ff;
                }
            } else if y <= self.prev_value.0 && y >= value1 {
                color |= 0x00ff;
            }

            let x = self.next_col;
            self.set_pixel_raw(x, y as u16, color, Buffer::Primary);
        }

        self.next_col = (self.next_col + 1) % WIDTH;
        self.prev_value = (value0, value1);
    }
}
