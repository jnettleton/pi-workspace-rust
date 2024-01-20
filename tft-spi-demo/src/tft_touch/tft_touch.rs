// use crate::tft_spi::TftSpi;
use crate::tft_spi::TftSpiImpl;

pub struct TftTouch {
    // tft_spi: Box<dyn TftSpi>,
    _tft_spi: TftSpiImpl,
}

impl TftTouch {
    // pub(crate) fn new(tft_spi: Box<dyn TftSpi>) -> Self {
    pub(crate) fn _new(tft_spi: TftSpiImpl) -> Self {
        Self {
            _tft_spi: tft_spi,
        }
    }
}