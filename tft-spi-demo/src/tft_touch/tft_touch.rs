use crate::tft_spi::TftSpi;

pub struct TftTouch {
    tft_spi: Box<dyn TftSpi>,
}

impl TftTouch {
    pub(crate) fn new(tft_spi: Box<dyn TftSpi>) -> Self {
        Self {
            tft_spi,
        }
    }
}