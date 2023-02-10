pub mod rpi_display;

use std::{error::Error, result};

use rpi_display::rpi_tft_display::{color::Color, RpiTftDisplay};

fn main() -> result::Result<(), Box<dyn Error>> {
    let mut display = RpiTftDisplay::new();
    display.initialize()?;

    display.fill_screen(Color::WHITE)?;
    display.fill_screen(Color::RED)?;
    display.fill_screen(Color::GREEN)?;
    display.fill_screen(Color::BLUE)?;
    display.fill_screen(Color::BLACK)?;

    Ok(())
}
