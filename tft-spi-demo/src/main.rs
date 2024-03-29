pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
mod util;

use std::{error::Error, result, thread};
use std::time::Duration;

use tft_display::{color::Color, enums::Rotate, tft_display::TftDisplay};
use tft_spi::TftSpiImpl;
// use tft_spi::TftSpi;
//use tft_touch::tft_touch::TftTouch;

fn main() -> result::Result<(), Box<dyn Error>> {
    // let tft_spi: Box<dyn TftSpi> = Box::new(TftSpiImpl::new());
    let tft_spi = TftSpiImpl::new();

    // let _touch = TftTouch::new(tft_spi.clone());
    let mut display = TftDisplay::new(tft_spi);
    display.initialize()?;

    display.init_screen_size(0, 0, 320, 480);

    display.set_rotation(Rotate::Degrees0);
    display.fill_screen(Color::BLUE)?;
    display.fill_rectangle(0, 480-40, 120, 40, Color::GREEN)?;
    thread::sleep(Duration::from_secs(1));

    display.set_rotation(Rotate::Degrees90);
    display.fill_screen(Color::RED)?;
    display.fill_rectangle(0, 320-40, 120, 40, Color::GREEN)?;
    thread::sleep(Duration::from_secs(1));

    display.set_rotation(Rotate::Degrees180);
    display.fill_screen(Color::YELLOW)?;
    display.fill_rectangle(0, 480-40, 120, 40, Color::GREEN)?;
    thread::sleep(Duration::from_secs(1));

    display.set_rotation(Rotate::Degrees270);
    display.fill_screen(Color::MAGENTA)?;
    display.fill_rectangle(0, 320-40, 120, 40, Color::GREEN)?;
    thread::sleep(Duration::from_secs(1));

    display.set_rotation(Rotate::Degrees90);
    display.fill_screen(Color::BLACK)?;
    // display.draw_text(10, 10, "Hello, World!", Color::WHITE, Color::BLUE, 4);

    display.fill_rectangle(0, 280, 120, 40, Color::RED)?;
    // display.draw_text(12, 292, "Executed", Color::WHITE, Color::RED, 2);

    display.fill_rectangle(120, 280, 120, 40, Color::YELLOW)?;
    // // display.draw_text(126, 292, "Scheduled", Color::BLACK, Color::YELLOW, 2);

    display.fill_rectangle(240, 280, 120, 40, Color::BLACK)?;
    // // display.draw_text(252, 292, "Routines", Color::BLACK, Color::MAGENTA, 2);

    display.fill_rectangle(360, 280, 120, 40, Color::MAGENTA)?;
    // // display.draw_text(384, 292, "Scenes", Color::WHITE, Color::BLACK, 2);

    Ok(())
}
