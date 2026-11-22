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

    // Landscape with a title and a row of four labeled buttons along the bottom.
    display.set_rotation(Rotate::Degrees90)?;
    display.fill_screen(Color::BLACK)?;
    display.draw_text(10, 10, "Hello, World!", Color::WHITE, Color::BLUE, 4)?;
    display.draw_text(
        10,
        60,
        "5x8 font, size 2\nsecond line, size 2",
        Color::GREEN,
        Color::BLACK,
        2,
    )?;
    display.draw_text(10, 110, "size 1: !\"#$%&'()*+,-./0123456789:;<=>?@", Color::WHITE, Color::BLACK, 1)?;
    display.draw_text(10, 120, "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~", Color::WHITE, Color::BLACK, 1)?;

    const BUTTON_W: u16 = 120;
    const BUTTON_H: u16 = 40;
    const LABEL_SIZE: u8 = 2;
    let button_y = display.height() - BUTTON_H;
    for (i, (label, fg, bg)) in [
        ("Executed", Color::WHITE, Color::RED),
        ("Scheduled", Color::BLACK, Color::YELLOW),
        ("Routines", Color::WHITE, Color::GREY),
        ("Scenes", Color::BLACK, Color::MAGENTA),
    ]
    .into_iter()
    .enumerate()
    {
        let button_x = i as u16 * BUTTON_W;
        display.fill_rectangle(button_x, button_y, BUTTON_W, BUTTON_H, bg)?;

        // Center the label in the button
        let text_w = TftDisplay::<TftSpiImpl>::text_width(label, LABEL_SIZE);
        let label_x = button_x + BUTTON_W.saturating_sub(text_w) / 2;
        let label_y = button_y + (BUTTON_H - 8 * LABEL_SIZE as u16) / 2;
        display.draw_text(label_x, label_y, label, fg, bg, LABEL_SIZE)?;
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
