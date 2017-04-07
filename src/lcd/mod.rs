#![allow(dead_code)]

pub use self::color::Color;
pub use self::init::init;

use board::ltdc::Ltdc;
use embedded::interfaces::gpio::OutputPin;
use core::ptr;

mod init;
mod color;

pub struct Lcd {
    controller: &'static mut Ltdc,
    display_enable: OutputPin,
    backlight_enable: OutputPin,
    next_pixel: u32,
    next_col: u32,
    prev_value: (u32, u32),
}

impl Lcd {
    pub fn set_background_color(&mut self, color: Color) {
        self.controller
            .bccr
            .update(|r| r.set_bc(color.to_rgb()));
    }

    pub fn test_pixels(&mut self) {
        let colors = [0xffff, 0xcccc, 0x9999, 0x6666, 0x3333, 0x0, 0xff00, 0x00ff];

        // layer 1: horizontal stripes
        let addr: u32 = 0xC000_0000;
        for i in 0..272 {
            for j in 0..480 {
                let pixel = i * 480 + j;
                let pixel_color = (addr + pixel * 2) as *mut u16;
                unsafe { ptr::write_volatile(pixel_color, colors[(i / 10) as usize & 7]) };
            }
        }

        let colors = [0xcccc, 0x9999, 0x6666, 0x3333, 0x0, 0xff00, 0x00ff, 0xffff];

        // layer 2: vertical stripes
        let addr: u32 = 0xC000_0000 + (480 * 272 * 2);
        for i in 0..272 {
            for j in 0..480 {
                let pixel = i * 480 + j;
                let pixel_color = (addr + pixel * 2) as *mut u16;
                unsafe { ptr::write_volatile(pixel_color, colors[(j / 10) as usize & 7]) };
            }
        }
    }

    pub fn clear_screen(&mut self) {
        // layer 1
        let addr: u32 = 0xC000_0000;
        for i in 0..272 {
            for j in 0..480 {
                let pixel = i * 480 + j;
                let pixel_color = (addr + pixel * 2) as *mut u16;
                unsafe { ptr::write_volatile(pixel_color, 0) };
            }
        }

        // layer 2
        let addr: u32 = 0xC000_0000 + (480 * 272 * 2);
        for i in 0..272 {
            for j in 0..480 {
                let pixel = i * 480 + j;
                let pixel_color = (addr + pixel * 2) as *mut u16;
                unsafe { ptr::write_volatile(pixel_color, 0) };
            }
        }
    }

    pub fn set_next_pixel(&mut self, color: u16) {
        // layer 1
        let addr: u32 = 0xC000_0000;
        let pixel_color = (addr + self.next_pixel * 2) as *mut u16;
        unsafe { ptr::write_volatile(pixel_color, color) };

        self.next_pixel = (self.next_pixel + 1) % (272 * 480);
    }

    pub fn set_next_col(&mut self, value0: u32, value1: u32) {
        let value0 = value0 + 2u32.pow(15);
        let value0 = value0 as u16 as u32;
        let value0 = value0 / 241;

        let value1 = value1 + 2u32.pow(15);
        let value1 = value1 as u16 as u32;
        let value1 = value1 / 241;

        // layer 1
        let addr: u32 = 0xC000_0000;
        for i in 0..272 {
            let mut color = 0;

            if value0 >= self.prev_value.0 {
                if i >= self.prev_value.0 && i <= value0 {
                    color |= 0xff00;
                }
            } else if i <= self.prev_value.0 && i >= value0 {
                color |= 0xff00;
            }

            if value1 >= self.prev_value.1 {
                if i >= self.prev_value.0 && i <= value1 {
                    color |= 0x00ff;
                }
            } else if i <= self.prev_value.0 && i >= value1 {
                color |= 0x00ff;
            }

            let pixel = i * 480 + self.next_col;
            let pixel_color = (addr + pixel * 2) as *mut u16;
            unsafe { ptr::write_volatile(pixel_color, color) };
        }


        self.next_col = (self.next_col + 1) % 480;
        self.prev_value = (value0, value1);
    }

    pub fn print_point_at(&mut self, x: u16, y: u16) {
        assert!(x < 480);
        assert!(y < 272);

        // layer 2
        let addr: u32 = 0xC000_0000 + (480 * 272 * 2);
        let pixel = u32::from(y) * 480 + u32::from(x);
        let pixel_color = (addr + pixel * 2) as *mut u16;

        unsafe { ptr::write_volatile(pixel_color, 0xffff) };
    }

    pub fn print_point_color_at(&mut self, x: u16, y: u16, color: u16) {
        assert!(x < 480);
        assert!(y < 272);

        // layer 2
        let addr: u32 = 0xC000_0000 + (480 * 272 * 2);
        let pixel = u32::from(y) * 480 + u32::from(x);
        let pixel_color = (addr + pixel * 2) as *mut u16;

        unsafe { ptr::write_volatile(pixel_color, color) };
    }
}
