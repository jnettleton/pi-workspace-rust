pub mod rpi_display;
pub mod rpi_spi;
pub mod rpi_touch;

use std::{error::Error, result};

use rpi_display::{color::Color, rpi_tft_display::RpiTftDisplay};
use rpi_spi::RpiSpi;
use rpi_touch::rpi_tft_touch::RpiTftTouch;

fn main() -> result::Result<(), Box<dyn Error>> {
    let rpi_spi:Box<dyn RpiSpi> = Box::RpiSpi::new();

    let _touch = RpiTftTouch::new(rpi_spi.clone());

    let mut display = RpiTftDisplay::new(rpi_spi.clone());
    display.initialize()?;

    display.fill_screen(Color::WHITE)?;
    display.fill_screen(Color::RED)?;
    display.fill_screen(Color::GREEN)?;
    display.fill_screen(Color::BLUE)?;
    display.fill_screen(Color::BLACK)?;

    Ok(())
}
