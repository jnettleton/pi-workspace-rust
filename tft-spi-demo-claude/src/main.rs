pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
#[allow(dead_code)]
mod util;

use std::{error::Error, thread, time::Duration};

use tft_display::{color::Color, tft_display::TftDisplay};
use tft_spi::TftSpiImpl;
use tft_touch::tft_touch::TftTouch;

fn main() -> Result<(), Box<dyn Error>> {
    // Display on CE0, touch on CE1: separate SPI handles, kernel handles chip select.
    let mut display = TftDisplay::new(TftSpiImpl::new()?);
    display.initialize()?;
    println!("display {}x{}", display.width(), display.height());

    for color in [Color::WHITE, Color::RED, Color::GREEN, Color::BLUE, Color::BLACK] {
        println!("fill {color}");
        display.fill_screen(color)?;
        thread::sleep(Duration::from_millis(500));
    }

    // Quick touch test: print raw readings for ~10 seconds.
    let mut touch = TftTouch::new()?;
    println!("touch the screen...");
    for _ in 0..200 {
        if let Some((x, y)) = touch.read_raw()? {
            println!("touch raw x={x} y={y}");
        }
        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}
