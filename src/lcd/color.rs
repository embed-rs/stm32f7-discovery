/// Represents a color with alpha, red, green, and blue channels.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    /// The red channel.
    pub red: u8,
    /// The green channel.
    pub green: u8,
    /// The blue channel.
    pub blue: u8,
    /// The alpha channel (0 is transparent, 255 is opaque).
    pub alpha: u8,
}

impl Color {
    /// Creates a color from the passed RGB values. The alpha channel is set to 255 (opaque).
    pub fn rgb(red: u8, green: u8, blue: u8) -> Color {
        Self::rgba(red, green, blue, 255)
    }

    /// Converts the color to RGB. The alpha channel is ignored.
    pub fn to_rgb(&self) -> u32 {
        (u32::from(self.red) << 16) | (u32::from(self.green) << 8) | u32::from(self.blue)
    }

    /// Creates a color from the passed values.
    pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color {
            red: red,
            green: green,
            blue: blue,
            alpha: alpha,
        }
    }

    /// Creates a color from the passed hex RGB value. The alpha channel is set to 255 (opaque).
    pub fn from_hex(color: u32) -> Color {
        assert_eq!(color >> (8 * 3), 0);
        Color {
            red: (color >> 16) as u8,
            green: (color >> 8) as u8,
            blue: color as u8,
            alpha: 255,
        }
    }

    /// Converts the color to RGB. The alpha channel is ignored.
    pub fn to_rgb888(&self) -> u32 {
        self.to_rgb()
    }

    /// Creates a color from the passed RGB value. The alpha channel is set to 255 (opaque).
    pub fn from_rgb888(color: u32) -> Color {
        Color::from_hex(color)
    }

    /// Converts the color to ARGB.
    pub fn to_argb8888(&self) -> u32 {
        (u32::from(self.alpha) << 24) | self.to_rgb888()
    }

    /// Creates a color from the passed ARGB value.
    pub fn from_argb8888(color: u32) -> Color {
        Color {
            red: (color >> 16) as u8,
            green: (color >> 8) as u8,
            blue: color as u8,
            alpha: (color >> 24) as u8,
        }
    }

    /// Converts the color to ARGB1555.
    pub fn to_argb1555(&self) -> u16 {
        (u16::from(self.alpha) & 0x80) << 8
            | (u16::from(self.red) & 0xf8) << 7
            | (u16::from(self.green) & 0xf8) << 2
            | (u16::from(self.blue) & 0xf8) >> 3
    }

    /// Creates a color from the passed ARGB1555 value.
    pub fn from_argb1555(color: u16) -> Color {
        Color {
            alpha: ((color >> 8) & 0x80) as u8,
            red: ((color >> 7) & 0xf8) as u8,
            green: ((color >> 2) & 0xf8) as u8,
            blue: ((color << 3) & 0xf8) as u8,
        }
    }

    /// Creates a color from the passed HSV value.
    pub fn from_hsv(hue: i32, saturation: f32, value: f32) -> Color {
        let mut h = hue % 360;
        if h < 0 {
            h += 360;
        }

        let c = value * saturation;
        let x = (h as i32 % 120) as f32 / 60f32 - 1f32;
        let x = c * (1f32 - if x < 0f32 { -x } else { x });
        let m = value - c;

        let mut rgb = (0f32, 0f32, 0f32);
        if h < 60 {
            rgb.0 = c;
            rgb.1 = x;
        } else if h < 120 {
            rgb.0 = x;
            rgb.1 = c;
        } else if h < 180 {
            rgb.1 = c;
            rgb.2 = x;
        } else if h < 240 {
            rgb.1 = x;
            rgb.2 = c;
        } else if h < 300 {
            rgb.0 = x;
            rgb.2 = c;
        } else {
            rgb.0 = c;
            rgb.2 = x;
        }

        rgb.0 += m;
        rgb.1 += m;
        rgb.2 += m;

        Color::rgb(
            (255f32 * rgb.0) as u8,
            (255f32 * rgb.1) as u8,
            (255f32 * rgb.2) as u8,
        )
    }
}
