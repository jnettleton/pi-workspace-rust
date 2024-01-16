pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
mod util;

use std::{error::Error, result};

use tft_display::{color::Color, tft_display::TftDisplay};
use tft_spi::{TftSpi, TftSpiImpl};
use tft_touch::tft_touch::TftTouch;

fn main() -> result::Result<(), Box<dyn Error>> {
    let tft_spi: Box<dyn TftSpi> = Box::new(TftSpiImpl::new());

    let _touch = TftTouch::new(tft_spi.clone());

    let mut display = TftDisplay::new(tft_spi.clone());
    display.initialize()?;

    display.fill_screen(Color::WHITE)?;
    display.fill_screen(Color::RED)?;
    display.fill_screen(Color::GREEN)?;
    display.fill_screen(Color::BLUE)?;
    display.fill_screen(Color::BLACK)?;

    Ok(())
}
