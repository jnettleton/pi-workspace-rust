pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;

use std::{error::Error, result};

use tft_display::{color::Color, tft_display::TftDisplay};
use tft_spi::TftSpi;
use tft_touch::tft_touch::TftTouch;

fn main() -> result::Result<(), Box<dyn Error>> {
    let rpi_spi:Box<dyn TftSpi> = Box::RpiSpi::new();

    let _touch = TftTouch::new(rpi_spi.clone());

    let mut display = TftDisplay::new(rpi_spi.clone());
    display.initialize()?;

    display.fill_screen(Color::WHITE)?;
    display.fill_screen(Color::RED)?;
    display.fill_screen(Color::GREEN)?;
    display.fill_screen(Color::BLUE)?;
    display.fill_screen(Color::BLACK)?;

    Ok(())
}
