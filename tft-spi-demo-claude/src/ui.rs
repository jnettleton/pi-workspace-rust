//! The demo's touch screen: a title, some text and a row of buttons along
//! the bottom of a landscape screen.

use crate::tft_display::{
    color::Color,
    tft_display::{Result, TftDisplay},
};
use crate::tft_spi::TftSpi;

pub const BUTTON_W: u16 = 120;
pub const BUTTON_H: u16 = 40;
pub const BUTTON_RADIUS: u16 = 8;
/// Black gap around each button
pub const BUTTON_GAP: u16 = 2;
pub const LABEL_SIZE: u8 = 2;
/// (label, text color, button color)
pub const BUTTONS: [(&str, Color, Color); 4] = [
    ("Executed", Color::WHITE, Color::RED),
    ("Scheduled", Color::BLACK, Color::YELLOW),
    ("Routines", Color::WHITE, Color::GREY),
    ("Scenes", Color::BLACK, Color::MAGENTA),
];

/// Clears the screen and draws a title, some sample text and the buttons.
pub fn draw_ui<S: TftSpi>(display: &mut TftDisplay<S>) -> Result<()> {
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
pub fn draw_button<S: TftSpi>(display: &mut TftDisplay<S>, index: usize, pressed: bool) -> Result<()> {
    let (label, text, button) = BUTTONS[index];
    let (fg, bg) = if pressed { (button, text) } else { (text, button) };

    let button_x = index as u16 * BUTTON_W;
    let button_y = display.height() - BUTTON_H;
    // Rounded button with a black gap around it
    display.fill_rectangle(button_x, button_y, BUTTON_W, BUTTON_H, Color::BLACK)?;
    display.fill_round_rectangle(
        i32::from(button_x + BUTTON_GAP),
        i32::from(button_y + BUTTON_GAP),
        BUTTON_W - 2 * BUTTON_GAP,
        BUTTON_H - 2 * BUTTON_GAP,
        BUTTON_RADIUS,
        bg,
    )?;

    // Center the label in the button. text_width includes the 1-column gap
    // after the last character, which isn't visible, so leave it out.
    let visible_w = TftDisplay::<S>::text_width(label, LABEL_SIZE).saturating_sub(u16::from(LABEL_SIZE));
    let label_x = button_x + BUTTON_W.saturating_sub(visible_w) / 2;
    let label_y = button_y + (BUTTON_H - 8 * u16::from(LABEL_SIZE)) / 2;
    display.draw_text(label_x, label_y, label, fg, bg, LABEL_SIZE)?;
    Ok(())
}

/// Which button a touch at screen (x, y) is on, if any, for a screen
/// `screen_height` pixels tall. The whole bottom strip counts, gaps included.
pub fn button_at(x: u16, y: u16, screen_height: u16) -> Option<usize> {
    if y < screen_height.saturating_sub(BUTTON_H) || y >= screen_height {
        return None;
    }
    Some(usize::from(x / BUTTON_W).min(BUTTONS.len() - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tft_display::{enums::Rotate, mock::MockSpi};

    fn landscape() -> (TftDisplay<MockSpi>, MockSpi) {
        let spi = MockSpi::new(480, 320);
        let mut display = TftDisplay::new(spi.clone());
        display.set_rotation(Rotate::Degrees90).unwrap();
        (display, spi)
    }

    fn pixel(spi: &MockSpi, x: u16, y: u16) -> [u8; 3] {
        let p = spi.0.borrow();
        p.pixels[usize::from(y) * p.width + usize::from(x)]
    }

    /// Pixels of `color` inside button `index`, as (x, y). Only the band
    /// between the rounded corners is searched, so the black gap and corners
    /// don't count (some labels are black).
    fn pixels_in_button(spi: &MockSpi, index: usize, color: Color) -> Vec<(u16, u16)> {
        let (x0, y0) = (index as u16 * BUTTON_W, 320 - BUTTON_H);
        let inset = BUTTON_GAP + BUTTON_RADIUS;
        let mut found = Vec::new();
        for y in y0 + inset..y0 + BUTTON_H - inset {
            for x in x0 + BUTTON_GAP..x0 + BUTTON_W - BUTTON_GAP {
                if pixel(spi, x, y) == color.to_rgb666_bytes() {
                    found.push((x, y));
                }
            }
        }
        found
    }

    fn button_area(spi: &MockSpi, index: usize) -> Vec<[u8; 3]> {
        let x0 = index as u16 * BUTTON_W;
        (320 - BUTTON_H..320)
            .flat_map(|y| (x0..x0 + BUTTON_W).map(move |x| (x, y)))
            .map(|(x, y)| pixel(spi, x, y))
            .collect()
    }

    #[test]
    fn buttons_along_the_bottom() {
        let (mut display, spi) = landscape();
        draw_ui(&mut display).unwrap();
        let black = Color::BLACK.to_rgb666_bytes();

        for (index, (label, text, button)) in BUTTONS.iter().enumerate() {
            let (bx, by) = (index as u16 * BUTTON_W, 320 - BUTTON_H);
            let name = format!("button {index} ({label})");

            // Filled with its color inside, black gap at the edges, rounded corners
            assert_eq!(pixel(&spi, bx + 4, by + 20), button.to_rgb666_bytes(), "{name}: fill");
            assert_eq!(pixel(&spi, bx, by + 20), black, "{name}: left gap");
            assert_eq!(pixel(&spi, bx + BUTTON_W - 1, by + 20), black, "{name}: right gap");
            assert_eq!(pixel(&spi, bx + 4, by), black, "{name}: top gap");
            assert_eq!(pixel(&spi, bx + BUTTON_GAP, by + BUTTON_GAP), black, "{name}: rounded corner");

            // Label in the text color, centered
            let label_pixels = pixels_in_button(&spi, index, *text);
            assert!(!label_pixels.is_empty(), "{name}: label drawn");
            let (min_x, max_x) = label_pixels.iter().fold((u16::MAX, 0), |(lo, hi), &(x, _)| (lo.min(x), hi.max(x)));
            let (min_y, max_y) = label_pixels.iter().fold((u16::MAX, 0), |(lo, hi), &(_, y)| (lo.min(y), hi.max(y)));
            let center_x = f64::from(min_x + max_x) / 2.0;
            let button_center_x = f64::from(bx) + f64::from(BUTTON_W - 1) / 2.0;
            assert!((center_x - button_center_x).abs() <= 0.5, "{name}: label centered at {center_x}");
            // Size-2 text is 16 rows tall, centered in the 40-row button: rows 12..28
            assert!(min_y >= by + 12 && max_y < by + 28, "{name}: label rows {min_y}..={max_y}");
        }
    }

    #[test]
    fn pressed_button_swaps_colors_and_restores() {
        let (mut display, spi) = landscape();
        draw_ui(&mut display).unwrap();
        let before: Vec<_> = (0..BUTTONS.len()).map(|i| button_area(&spi, i)).collect();

        for (index, (_, text, button)) in BUTTONS.iter().enumerate() {
            let (bx, by) = (index as u16 * BUTTON_W, 320 - BUTTON_H);
            draw_button(&mut display, index, true).unwrap();
            assert_eq!(pixel(&spi, bx + 4, by + 20), text.to_rgb666_bytes(), "button {index}: pressed fill");
            assert!(!pixels_in_button(&spi, index, *button).is_empty(), "button {index}: pressed label");
            // Only this button changed
            for other in (0..BUTTONS.len()).filter(|&i| i != index) {
                assert!(button_area(&spi, other) == before[other], "button {other} changed");
            }

            draw_button(&mut display, index, false).unwrap();
            assert!(button_area(&spi, index) == before[index], "button {index} not restored");
        }
    }

    #[test]
    fn button_hit_testing() {
        let h = 320;
        assert_eq!(button_at(0, 279, h), None, "just above the buttons");
        assert_eq!(button_at(0, 280, h), Some(0));
        assert_eq!(button_at(119, 319, h), Some(0));
        assert_eq!(button_at(120, 300, h), Some(1));
        assert_eq!(button_at(240, 300, h), Some(2));
        assert_eq!(button_at(359, 300, h), Some(2));
        assert_eq!(button_at(360, 300, h), Some(3));
        assert_eq!(button_at(479, 319, h), Some(3));
        assert_eq!(button_at(600, 300, h), Some(3), "past the right edge counts as the last button");
        assert_eq!(button_at(10, 320, h), None, "below the screen");
        assert_eq!(button_at(10, 10, 20), Some(0), "short screen: buttons fill it");
    }
}
