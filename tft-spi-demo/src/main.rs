pub mod rpi_display;

use std::{error::Error, result, thread, time::Duration};

// use rpi_display::rpi_spi::RpiSpi;
use rpi_display::rpi_tft_display::{color::Color, Result, RpiTftDisplay};

fn main() -> result::Result<(), Box<dyn Error>> {
    // let _spi = RpiSpi::new();

    let mut display = RpiTftDisplay::new();
    display.initialize()?;

    for _ in 0..3 {
        blink_display(&mut display)?;
    }

    Ok(())
}

fn blink_display(display: &mut RpiTftDisplay) -> Result<()> {
    display.fill_screen(Color::WHITE)?;
    thread::sleep(Duration::from_secs(1));

    display.fill_screen(Color::BLACK)?;
    thread::sleep(Duration::from_secs(1));

    Ok(())
}
