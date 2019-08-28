//! Functions for accessing and writing text to the LCD.
//!
//! The display has two layers that are blended on top of each other, and a background layer
//! with an uniform color.

pub use self::color::Color;
pub use self::init::init;
pub use self::stdout::init as init_stdout;

use core::fmt;
use stm32f7::stm32f7x6::LTDC;

#[macro_use]
pub mod stdout;
mod color;
mod init;

/// The height of the display in pixels.
pub const HEIGHT: usize = 272;
/// The width of the display in pixels.
pub const WIDTH: usize = 480;

/// The number of bytes per pixel for layer 1.
pub const LAYER_1_OCTETS_PER_PIXEL: usize = 4;
/// The length of the layer 1 buffer in bytes.
pub const LAYER_1_LENGTH: usize = HEIGHT * WIDTH * LAYER_1_OCTETS_PER_PIXEL;
/// The number of bytes per pixel for layer 2.
pub const LAYER_2_OCTETS_PER_PIXEL: usize = 2;
/// The length of the layer 1 buffer in bytes.
pub const LAYER_2_LENGTH: usize = HEIGHT * WIDTH * LAYER_2_OCTETS_PER_PIXEL;

/// Represents the LCD and provides methods to access both layers.
pub struct Lcd<'a> {
    controller: &'a mut LTDC,

    /// A layer with RGB + alpha value
    ///
    /// Use `.take()` to get an owned version of this layer.
    pub layer_1: Option<Layer<FramebufferArgb8888>>,

    /// A layer with alpha + color lookup table index
    ///
    /// Use `.take()` to get an owned version of this layer.
    pub layer_2: Option<Layer<FramebufferAl88>>,
}

impl<'a> Lcd<'a> {
    fn new(
        ltdc: &'a mut LTDC,
        layer_1: &'static mut [volatile::Volatile<u8>],
        layer_2: &'static mut [volatile::Volatile<u8>],
    ) -> Self {
        Self {
            controller: ltdc,
            layer_1: Some(Layer { framebuffer: FramebufferArgb8888::new(layer_1) } ),
            layer_2: Some(Layer { framebuffer: FramebufferAl88::new(layer_2) } ),
        }
    }

    /// Sets the color of the background layer.
    pub fn set_background_color(&mut self, color: Color) {
        self.controller
            .bccr
            .modify(|_, w| unsafe { w.bc().bits(color.to_rgb()) });
    }

    /// Sets the color `i` in the lookup table for layer 2
    pub fn set_color_lookup_table(&mut self, i: u8, color: Color) {
        self.controller
            .l2clutwr
            .write(|w| unsafe { w
                .clutadd().bits(i)
                .red().bits(color.red)
                .green().bits(color.green)
                .blue().bits(color.blue)
            });
    }

    fn reload_shadow_registers(&mut self) {
        self.controller.srcr.modify(|_, w| w.imr().set_bit()); // IMMEDIATE_RELOAD
    }
}

/// Represents a buffer of pixels.
pub trait Framebuffer {
    /// Set the pixel at the specified coordinates to the specified color.
    fn set_pixel(&mut self, x: usize, y: usize, color: Color);
}

/// A framebuffer in the ARGB8888 format.
///
/// It uses 8bits for alpha, red, green, and black respectively, totaling in 32bits per pixel.
pub struct FramebufferArgb8888 {
    mem: &'static mut [volatile::Volatile<u8>],
}

impl FramebufferArgb8888 {
    fn new(mem: &'static mut [volatile::Volatile<u8>]) -> Self {
        Self { mem }
    }
}

impl Framebuffer for FramebufferArgb8888 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * WIDTH + x;
        let pixel_idx = pixel * LAYER_1_OCTETS_PER_PIXEL;
        self.mem[pixel_idx].write(color.alpha);
        self.mem[pixel_idx + 1].write(color.red);
        self.mem[pixel_idx + 2].write(color.green);
        self.mem[pixel_idx + 3].write(color.blue);
    }
}

/// A framebuffer in the AL88 format.
///
/// There are 8bits for the alpha channel and 8 bits for specifying a color using a
/// lookup table. Thus, each pixel is represented by 16bits.
pub struct FramebufferAl88 {
    mem: &'static mut [volatile::Volatile<u8>],
}

impl FramebufferAl88 {
    fn new(mem: &'static mut [volatile::Volatile<u8>]) -> Self {
        Self { mem }
    }
}

impl Framebuffer for FramebufferAl88 {
    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let pixel = y * WIDTH + x;
        let pixel_idx = pixel * LAYER_2_OCTETS_PER_PIXEL;
        self.mem[pixel_idx].write(color.alpha);
        self.mem[pixel_idx + 1].write(color.red);
    }
}

/// Represents a layer of the LCD controller.
pub struct Layer<T> {
    framebuffer: T,
}

impl<T: Framebuffer> Layer<T> {
    /// Fill the layer with horizontal stripes.
    ///
    /// Useful for testing.
    pub fn horizontal_stripes(&mut self) {
        let colors = [
            0xff_ff_ff, 0xcc_cc_cc, 0x99_99_99, 0x66_66_66, 0x33_33_33, 0x00_00_00, 0xff_00_00, 0x00_00_ff,
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

    /// Fill the layer with vertical stripes.
    ///
    /// Useful for testing.
    pub fn vertical_stripes(&mut self) {
        let colors = [
            0xcc_cc_cc, 0x99_99_99, 0x66_66_66, 0x33_33_33, 0x00_00_00, 0xff_00_00, 0x00_00_ff, 0xff_ff_ff,
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

    /// Clear all pixels.
    ///
    /// This method sets each pixel to transparent or black, depending on the framebuffer format.
    pub fn clear(&mut self) {
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                self.framebuffer.set_pixel(j, i, Color::from_argb8888(0));
            }
        }
    }

    /// Sets the pixel at the specified coordinates to white.
    pub fn print_point_at(&mut self, x: usize, y: usize) {
        self.print_point_color_at(x, y, Color::from_hex(0xff_ff_ff));
    }

    /// Sets the pixel at the specified coordinates to the specified color.
    pub fn print_point_color_at(&mut self, x: usize, y: usize, color: Color) {
        assert!(x < WIDTH);
        assert!(y < HEIGHT);

        self.framebuffer.set_pixel(x, y, color);
    }

    /// Creates a text writer on this layer.
    pub fn text_writer(&mut self) -> TextWriter<T> {
        TextWriter {
            layer: self,
            x_pos: 0,
            y_pos: 0,
        }
    }
}

/// Allows to print audio data.
pub struct AudioWriter {
    next_pixel: usize,
    next_col: usize,
    prev_value: (usize, usize),
}

impl AudioWriter {
    /// Creates a new audio writer starting at the left edge of the screen.
    pub const fn new() -> Self {
        AudioWriter {
            next_pixel: 0,
            next_col: 0,
            prev_value: (0, 0),
        }
    }

    /// Sets the next pixel on the layer.
    ///
    /// Useful for testing.
    pub fn set_next_pixel<F: Framebuffer>(&mut self, layer: &mut Layer<F>, color: Color) {
        layer.print_point_color_at(self.next_pixel % WIDTH, self.next_pixel / WIDTH, color);
        self.next_pixel = (self.next_pixel + 1) % (HEIGHT * WIDTH);
    }

    /// Sets the next column of the screen according to the passed audio data.
    pub fn set_next_col<F: Framebuffer>(&mut self, layer: &mut Layer<F>, value0: u32, value1: u32) {
        let value0 = value0 + 2u32.pow(15);
        let value0 = value0 as u16 as usize;
        let value0 = value0 / 241;

        let value1 = value1 + 2u32.pow(15);
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
            layer.print_point_color_at(self.next_col, i, color);
        }

        self.next_col = (self.next_col + 1) % WIDTH;
        self.prev_value = (value0, value1);
    }
}

/// Allows writing text to the wrapped layer.
///
/// This struct implements the [fmt::Write](core::fmt::Write) trait, which makes it possible
/// to use the `writeln!` macro with this struct.
pub struct TextWriter<'a, T: Framebuffer + 'a> {
    layer: &'a mut Layer<T>,
    /// Column position of the cursor
    pub x_pos: usize,
    /// Row/Line position of the cursor
    pub y_pos: usize,
}

impl<'a, T: Framebuffer> TextWriter<'a, T> {
    fn newline(&mut self) {
        self.y_pos += 8;
        self.carriage_return()
    }
    fn carriage_return(&mut self) {
        self.x_pos = 0;
    }
    /// Erases all text on the screen
    pub fn clear(&mut self) {
        self.x_pos = 0;
        self.y_pos = 0;
        self.layer.clear();
    }
}

impl<'a, T: Framebuffer> fmt::Write for TextWriter<'a, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use font8x8::UnicodeFonts;

        for c in s.chars() {
            if c == '\n' {
                self.newline();
                continue;
            } else if c == '\r' {
                self.carriage_return();
                continue;
            }
            match c {
                ' '..='~' => {
                    if self.x_pos >= WIDTH {
                        self.newline();
                    }
                    if self.y_pos >= HEIGHT {
                        self.clear();
                    }
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
        }
        Ok(())
    }
}
