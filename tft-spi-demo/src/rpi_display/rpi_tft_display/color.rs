/// 6-bit RGB color
pub struct Color(u8, u8, u8);

impl Color {
    const WHITE: Color = Color(63, 63, 63);
    const BLACK: Color = Color(0, 0, 0);

    pub fn from_eight_bit(r: u8, g: u8, b: u8) -> Self {
        Self(
            Self::eight_to_six_bit(r),
            Self::eight_to_six_bit(g),
            Self::eight_to_six_bit(b),
        )
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
