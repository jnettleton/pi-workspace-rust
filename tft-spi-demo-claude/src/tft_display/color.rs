use std::fmt::Display;

/// 24-bit RGB color, sent to the panel as RGB565.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Converts 24-bit color (e.g. white 0xFFFFFF) to `Color`
    ///
    /// # Errors
    ///
    /// Errors if input is greater than 0xFFFFFF
    pub fn from_24_bit(color: u32) -> Result<Self, &'static str> {
        if color > 0xFF_FF_FF {
            return Err("color input too large");
        }

        Ok(Self::rgb((color >> 16) as u8, (color >> 8) as u8, color as u8))
    }

    pub fn red(&self) -> u8 {
        self.r
    }

    pub fn green(&self) -> u8 {
        self.g
    }

    pub fn blue(&self) -> u8 {
        self.b
    }

    /// RGB565: rrrrrggg gggbbbbb
    pub fn to_rgb565(&self) -> u16 {
        ((self.r as u16 & 0xF8) << 8) | ((self.g as u16 & 0xFC) << 3) | (self.b as u16 >> 3)
    }

    /// RGB565 as the two bytes sent over SPI (big-endian).
    pub fn to_be_bytes(&self) -> [u8; 2] {
        self.to_rgb565().to_be_bytes()
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.red(), self.green(), self.blue())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_conversion() {
        assert_eq!(Color::WHITE.to_rgb565(), 0xFFFF);
        assert_eq!(Color::BLACK.to_rgb565(), 0x0000);
        assert_eq!(Color::RED.to_rgb565(), 0xF800);
        assert_eq!(Color::GREEN.to_rgb565(), 0x07E0);
        assert_eq!(Color::BLUE.to_rgb565(), 0x001F);
        assert_eq!(Color::RED.to_be_bytes(), [0xF8, 0x00]);
    }
}
