mod calibrate;
pub mod tft_display;
pub mod tft_spi;
pub mod tft_touch;
#[allow(dead_code)]
mod util;

use std::{env, error::Error, path::Path, thread, time::Duration};

use tft_display::{color::Color, enums::Rotate, tft_display::TftDisplay};
use tft_spi::TftSpiImpl;
use tft_touch::tft_touch::TftTouch;

type Display = TftDisplay<TftSpiImpl>;

/// Saved in the current directory. Run with `--calibrate` to redo it.
const CALIBRATION_FILE: &str = "touch_calibration.txt";

const BUTTON_W: u16 = 120;
const BUTTON_H: u16 = 40;
const LABEL_SIZE: u8 = 2;
/// (label, text color, button color)
const BUTTONS: [(&str, Color, Color); 4] = [
    ("Executed", Color::WHITE, Color::RED),
    ("Scheduled", Color::BLACK, Color::YELLOW),
    ("Routines", Color::WHITE, Color::GREY),
    ("Scenes", Color::BLACK, Color::MAGENTA),
];

fn main() -> Result<(), Box<dyn Error>> {
    let force_calibration = env::args().any(|arg| arg == "--calibrate");

    // Display on CE0, touch on CE1: separate SPI handles, kernel handles chip select.
    let mut display = TftDisplay::new(TftSpiImpl::new()?);
    display.initialize()?;
    let mut touch = TftTouch::new()?;

    rotation_test(&mut display)?;

    // Landscape UI. The calibration only applies to the rotation it was made in.
    display.set_rotation(Rotate::Degrees90)?;
    let calibration = calibrate::load_or_calibrate(
        &mut display,
        &mut touch,
        Path::new(CALIBRATION_FILE),
        force_calibration,
    )?;

    draw_ui(&mut display)?;
    let button_y = display.height() - BUTTON_H;

    // Touch to draw dots; touch a button to highlight it. Ctrl-C to quit.
    println!("touch the screen (Ctrl-C to quit)...");
    loop {
        let Some(raw) = touch.read_filtered(TftTouch::FILTER_SAMPLES)? else {
            thread::sleep(Duration::from_millis(10));
            continue;
        };
        let (x, y) = calibration.to_screen(raw, display.width(), display.height());

        if y >= button_y {
            let index = usize::from(x / BUTTON_W).min(BUTTONS.len() - 1);
            println!("touch ({x}, {y}): button {}", BUTTONS[index].0);
            draw_button(&mut display, index, true)?;
            touch.wait_for_release();
            draw_button(&mut display, index, false)?;
        } else {
            println!("touch ({x}, {y}) raw {raw:?}");
            let (dot_x, dot_y) = (x.saturating_sub(1), y.saturating_sub(1).min(button_y - 3));
            display.fill_rectangle(dot_x, dot_y, 3, 3, Color::WHITE)?;
        }
    }
}

/// Each orientation gets a background color and a green block in its
/// bottom-left corner.
fn rotation_test(display: &mut Display) -> Result<(), Box<dyn Error>> {
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
    Ok(())
}

/// A title, some sample text and a row of buttons along the bottom.
fn draw_ui(display: &mut Display) -> Result<(), Box<dyn Error>> {
    display.fill_screen(Color::BLACK)?;
    display.draw_text(10, 10, "Hello, World!", Color::WHITE, Color::BLUE, 4)?;
    display.draw_text(
        10,
        60,
        "Touch to draw, tap a button\nRun with --calibrate to redo",
        Color::GREEN,
        Color::BLACK,
        2,
    )?;

    for index in 0..BUTTONS.len() {
        draw_button(display, index, false)?;
    }
    Ok(())
}

/// Draws a button; `pressed` swaps its text and background colors.
fn draw_button(display: &mut Display, index: usize, pressed: bool) -> Result<(), Box<dyn Error>> {
    let (label, text, button) = BUTTONS[index];
    let (fg, bg) = if pressed { (button, text) } else { (text, button) };

    let button_x = index as u16 * BUTTON_W;
    let button_y = display.height() - BUTTON_H;
    display.fill_rectangle(button_x, button_y, BUTTON_W, BUTTON_H, bg)?;

    // Center the label in the button
    let text_w = Display::text_width(label, LABEL_SIZE);
    let label_x = button_x + BUTTON_W.saturating_sub(text_w) / 2;
    let label_y = button_y + (BUTTON_H - 8 * u16::from(LABEL_SIZE)) / 2;
    display.draw_text(label_x, label_y, label, fg, bg, LABEL_SIZE)?;
    Ok(())
}
