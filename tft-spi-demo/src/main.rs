pub mod rpi_display;

// use rpi_display::rpi_spi::RpiSpi;
use rpi_display::{enums::ST7735Color, rpi_tft_display::RpiTftDisplay};
use std::io;

fn main() -> io::Result<()> {
    // let _spi = RpiSpi::new();

    let mut display = RpiTftDisplay::new();
    display.initialize()?;
    display.fill_screen(ST7735Color::RED as u16)?;

    Ok(())
}
