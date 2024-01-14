use crate::rpi_spi::RpiSpi;

pub struct RpiTftTouch {
    rpi_spi: Box<dyn RpiSpi>,
}

impl RpiTftTouch {
    pub(crate) fn new(rpi_spi: Box<dyn RpiSpi>) -> Self {
        let spi = RpiSpi::new();
        Self {
            rpi_spi,
        }
    }
}