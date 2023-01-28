pub mod rpi_display;

use std::io;
use rpi_display::rpi_tft_display::ST7735Color;
use rpi_display::rpi_tft_display::RpiTftDisplay;

fn main() -> io::Result<()> {
    let mut tft_display = RpiTftDisplay::new();
    tft_display.initialize()?;
    tft_display.fill_screen(ST7735Color::RED as u16)?;

    Ok(())
}
