use super::rpi_spi::RpiSpi;

pub struct RpiTftDisplay {
    rpi_spi: RpiSpi,
}

impl RpiTftDisplay {
    fn new() -> Self {
        let spi = RpiSpi::new();
        Self {
            rpi_spi: spi,
        }
    }
}