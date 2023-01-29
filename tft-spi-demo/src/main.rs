pub mod rpi_display;

use std::io;
use rpi_display::rpi_tft_display::ST7735Color;
use rpi_display::rpi_tft_display::RpiTftDisplay;
use rpi_display::rpi_spi::RpiSpi;

fn main() -> io::Result<()> {
    let _spi = RpiSpi::new();

    let mut display = RpiTftDisplay::new();
    display.initialize()?;
    display.fill_screen(ST7735Color::RED as u16)?;

    Ok(())
}
