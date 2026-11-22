use std::fmt::Display;

/// 24-bit RGB color, sent to the panel as 18-bit RGB666.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const TAN: Color = Color::rgb(210, 180, 140);
    pub const GREY: Color = Color::rgb(128, 128, 128);
    pub const BROWN: Color = Color::rgb(165, 42, 42);

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

    /// The 3 bytes sent over SPI in 18-bit mode. The ILI9486 reads the top
    /// 6 bits of each byte (D7..D2), so the 8-bit value is used as-is with the
    /// low 2 bits cleared.
    pub fn to_rgb666_bytes(&self) -> [u8; 3] {
        [self.r & 0xFC, self.g & 0xFC, self.b & 0xFC]
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
    fn rgb666_conversion() {
        assert_eq!(Color::WHITE.to_rgb666_bytes(), [0xFC, 0xFC, 0xFC]);
        assert_eq!(Color::BLACK.to_rgb666_bytes(), [0x00, 0x00, 0x00]);
        assert_eq!(Color::RED.to_rgb666_bytes(), [0xFC, 0x00, 0x00]);
        assert_eq!(Color::BLUE.to_rgb666_bytes(), [0x00, 0x00, 0xFC]);
        assert_eq!(Color::rgb(0x13, 0x26, 0x11).to_rgb666_bytes(), [0x10, 0x24, 0x10]);
    }

    #[test]
    fn from_24_bit() {
        assert_eq!(Color::from_24_bit(0xFF8000), Ok(Color::rgb(0xFF, 0x80, 0x00)));
        assert!(Color::from_24_bit(0x1_000_000).is_err());
    }
}
