pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
#[allow(dead_code)]
mod util;

use std::{error::Error, thread, time::Duration};

use tft_display::{color::Color, enums::Rotate, tft_display::TftDisplay};
use tft_spi::TftSpiImpl;
use tft_touch::tft_touch::TftTouch;

fn main() -> Result<(), Box<dyn Error>> {
    // Display on CE0, touch on CE1: separate SPI handles, kernel handles chip select.
    let mut display = TftDisplay::new(TftSpiImpl::new()?);
    display.initialize()?;

    // Rotation test: each orientation gets a background color and a green
    // block in its bottom-left corner.
    for (rotation, background) in [
        (Rotate::Degrees0, Color::BLUE),
        (Rotate::Degrees90, Color::RED),
        (Rotate::Degrees180, Color::YELLOW),
        (Rotate::Degrees270, Color::MAGENTA),
    ] {
        display.set_rotation(rotation)?;
        println!("{rotation:?}: {}x{}", display.width(), display.height());
        display.fill_screen(background)?;
        display.fill_rectangle(0, display.height() - 40, 120, 40, Color::GREEN)?;
        thread::sleep(Duration::from_secs(1));
    }

    // Landscape with a row of four buttons along the bottom.
    display.set_rotation(Rotate::Degrees90)?;
    display.fill_screen(Color::BLACK)?;
    let button_y = display.height() - 40;
    for (i, color) in [Color::RED, Color::YELLOW, Color::GREY, Color::MAGENTA]
        .into_iter()
        .enumerate()
    {
        display.fill_rectangle(i as u16 * 120, button_y, 120, 40, color)?;
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
