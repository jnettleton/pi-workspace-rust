use std::fmt::Display;

/// 18-bit RGB color
#[derive(Debug)]
pub struct Color(u8, u8, u8);

impl Color {
    pub const BLACK: Color = Color(0x00, 0x00, 0x00);
    pub const WHITE: Color = Color(0x3F, 0x3F, 0x3F);
    pub const RED: Color = Color(0x3F, 0x00, 0x00);
    pub const GREEN: Color = Color(0x00, 0x3F, 0x00);
    pub const BLUE: Color = Color(0x00, 0x00, 0x3F);
    pub const CYAN: Color = Color(0x00, 0x3F, 0x1F);
    pub const MAGENTA: Color = Color(0x1F, 0x00, 0x1F);
    pub const YELLOW: Color = Color(0x3F, 0x3E, 0x00);
    pub const TAN: Color = Color(0x1D, 0x10, 0x11);
    pub const GREY: Color = Color(0x13, 0x26, 0x11);
    pub const BROWN: Color = Color(0x10, 0x10, 0x01);

    pub fn from_eight_bit_rgb(r: u8, g: u8, b: u8) -> Self {
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

        Ok(Self::from_eight_bit_rgb(
            (color >> 16) as u8,
            (color >> 8) as u8,
            color as u8,
        ))
    }

    pub fn red(&self) -> u8 {
        self.2
    }

    pub fn green(&self) -> u8 {
        self.1
    }

    pub fn blue(&self) -> u8 {
        self.0
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
