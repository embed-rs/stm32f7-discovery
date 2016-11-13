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
        assert!(color >> (8 * 3) == 0);
        Color {
            red: (color >> 16) as u8,
            green: (color >> 8) as u8,
            blue: color as u8,
            alpha: 255,
        }
    }
}
