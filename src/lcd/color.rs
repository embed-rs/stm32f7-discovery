#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub fn rgb(red: u8, green: u8, blue: u8) -> Color {
        Self::rgba(red, green, blue, 255)
    }

    pub fn to_rgb(&self) -> u32 {
        (u32::from(self.red) << 16) | (u32::from(self.green) << 8) | u32::from(self.blue)
    }

    pub fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color {
            red: red,
            green: green,
            blue: blue,
            alpha: alpha,
        }
    }

    pub fn from_hex(color: u32) -> Color {
        assert_eq!(color >> (8 * 3), 0);
        Color {
            red: (color >> 16) as u8,
            green: (color >> 8) as u8,
            blue: color as u8,
            alpha: 255,
        }
    }

    pub fn to_rgb888(&self) -> u32 {
        self.to_rgb()
    }

    pub fn from_rgb888(color: u32) -> Color {
        Color::from_hex(color)
    }

    pub fn to_argb8888(&self) -> u32 {
        (u32::from(self.alpha) << 24) | self.to_rgb888()
    }

    pub fn from_argb8888(color: u32) -> Color {
        Color {
            red: (color >> 16) as u8,
            green: (color >> 8) as u8,
            blue: color as u8,
            alpha: (color >> 24) as u8,
        }
    }

    pub fn to_argb1555(&self) -> u16 {
        (u16::from(self.alpha) & 0x80) << 8
            | (u16::from(self.red) & 0xf8) << 7
            | (u16::from(self.green) & 0xf8) << 2
            | (u16::from(self.blue) & 0xf8) >> 3
    }

    pub fn from_argb1555(color: u16) -> Color {
        Color {
            alpha: ((color >> 8) & 0x80) as u8,
            red: ((color >> 7) & 0xf8) as u8,
            green: ((color >> 2) & 0xf8) as u8,
            blue: ((color << 3) & 0xf8) as u8
        }
    }
}
