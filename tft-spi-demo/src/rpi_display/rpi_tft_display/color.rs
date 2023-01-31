use std::fmt::Display;

/// 6-bit RGB color
#[derive(Debug)]
pub struct Color(u8, u8, u8);

impl Color {
    pub const WHITE: Color = Color(63, 63, 63);
    pub const BLACK: Color = Color(0, 0, 0);

    pub fn from_eight_bit(r: u8, g: u8, b: u8) -> Self {
        Self(
            Self::eight_to_six_bit(r),
            Self::eight_to_six_bit(g),
            Self::eight_to_six_bit(b),
        )
    }

    /// Converts 24-bit color (e.g. white 0xFFFFFF) to `Color`
    ///
    /// # Errors
    ///
    /// Errors if input is greater than 0xFFFFFF ()
    pub fn from_24_bit(color: u32) -> Result<Self, &'static str> {
        if color > 0xFF_FF_FF {
            return Err("color input too large");
        }

        let r = (color & 0xFF_00_00) >> 16;
        let g = (color & 0x00_FF_00) >> 8;
        let b = color & 0x00_00_FF;

        Ok(Self::from_eight_bit(r as u8, g as u8, b as u8))
    }

    pub fn red(&self) -> u8 {
        self.0
    }

    pub fn green(&self) -> u8 {
        self.1
    }

    pub fn blue(&self) -> u8 {
        self.2
    }

    fn eight_to_six_bit(input: u8) -> u8 {
        ((input as f32 / 255.0) * 63.0) as u8
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.red(), self.green(), self.blue())
    }
}