//! Interactive touch calibration, and saving/loading the result.
//!
//! The calibration file holds one line: the display rotation in degrees
//! followed by the six calibration coefficients, e.g.
//! `90 0.0012 -0.1301 509.6 0.0861 0.0004 -16.2`.

use std::{error::Error, fs, path::Path, thread, time::Duration};

use crate::tft_display::{color::Color, enums::Rotate, tft_display::TftDisplay};
use crate::tft_spi::TftSpi;
use crate::tft_touch::{
    calibration::{Calibration, CalibrationPoint},
    tft_touch::TftTouch,
};

type BoxResult<T> = Result<T, Box<dyn Error>>;

/// Distance of the corner targets from the screen edges
const TARGET_INSET: u16 = 30;
/// Half-length of the target crosshair arms
const TARGET_ARM: u16 = 10;
/// A calibration is accepted if every target maps back within this many pixels.
const MAX_ERROR_PX: f64 = 8.0;

/// Loads the calibration from `path` if it exists and matches the current
/// rotation; otherwise (or if `force` is set) runs [`run`] and saves the result.
pub fn load_or_calibrate<S: TftSpi>(
    display: &mut TftDisplay<S>,
    touch: &mut TftTouch,
    path: &Path,
    force: bool,
) -> BoxResult<Calibration> {
    if !force {
        if let Some(calibration) = load(path, display.rotation()) {
            println!("loaded touch calibration from {}", path.display());
            return Ok(calibration);
        }
        println!(
            "no touch calibration for {} degrees in {}, calibrating",
            display.rotation().degrees(),
            path.display()
        );
    }

    let calibration = run(display, touch)?;
    save(path, display.rotation(), &calibration)?;
    println!("saved touch calibration to {}", path.display());
    Ok(calibration)
}

/// Shows 5 targets one at a time (4 corners, then center) and fits a
/// calibration to where they were touched. Repeats until the fit is good.
pub fn run<S: TftSpi>(display: &mut TftDisplay<S>, touch: &mut TftTouch) -> BoxResult<Calibration> {
    let (w, h) = (display.width(), display.height());
    let targets = [
        (TARGET_INSET, TARGET_INSET),
        (w - 1 - TARGET_INSET, TARGET_INSET),
        (w - 1 - TARGET_INSET, h - 1 - TARGET_INSET),
        (TARGET_INSET, h - 1 - TARGET_INSET),
        (w / 2, h / 2),
    ];

    loop {
        display.fill_screen(Color::BLACK)?;
        draw_centered(display, h / 2 + 30, "Touch each target", Color::WHITE, 2)?;

        let mut points = Vec::with_capacity(targets.len());
        for &target in &targets {
            draw_target(display, target, Color::RED)?;
            let raw = touch.wait_for_touch()?;
            draw_target(display, target, Color::GREEN)?;
            touch.wait_for_release();
            draw_target(display, target, Color::BLACK)?;

            println!("target {target:?}: raw {raw:?}");
            points.push(CalibrationPoint { raw, screen: target });
        }

        let message = match Calibration::fit(&points) {
            Some(calibration) => {
                let error = calibration.max_error(&points);
                println!("calibration max error {error:.1} px");
                if error <= MAX_ERROR_PX {
                    display.fill_screen(Color::BLACK)?;
                    draw_centered(display, h / 2 - 8, "Calibrated", Color::GREEN, 2)?;
                    thread::sleep(Duration::from_millis(750));
                    return Ok(calibration);
                }
                format!("Off by {error:.0} px, try again")
            }
            None => "Bad readings, try again".to_string(),
        };

        display.fill_screen(Color::BLACK)?;
        draw_centered(display, h / 2 - 8, &message, Color::YELLOW, 2)?;
        thread::sleep(Duration::from_millis(1500));
    }
}

fn load(path: &Path, rotation: Rotate) -> Option<Calibration> {
    let text = fs::read_to_string(path).ok()?;
    let (degrees, coefficients) = text.trim().split_once(' ')?;
    let saved_rotation = Rotate::from_degrees(degrees.parse().ok()?)?;
    if saved_rotation != rotation {
        return None;
    }
    Calibration::from_text(coefficients)
}

fn save(path: &Path, rotation: Rotate, calibration: &Calibration) -> std::io::Result<()> {
    fs::write(
        path,
        format!("{} {}\n", rotation.degrees(), calibration.to_text()),
    )
}

fn draw_target<S: TftSpi>(
    display: &mut TftDisplay<S>,
    (x, y): (u16, u16),
    color: Color,
) -> BoxResult<()> {
    let arm_len = 2 * TARGET_ARM + 1;
    display.fill_rectangle(x - TARGET_ARM, y, arm_len, 1, color)?;
    display.fill_rectangle(x, y - TARGET_ARM, 1, arm_len, color)?;
    Ok(())
}

fn draw_centered<S: TftSpi>(
    display: &mut TftDisplay<S>,
    y: u16,
    text: &str,
    color: Color,
    size: u8,
) -> BoxResult<()> {
    let text_w = TftDisplay::<S>::text_width(text, size);
    let x = display.width().saturating_sub(text_w) / 2;
    display.draw_text(x, y, text, color, Color::BLACK, size)?;
    Ok(())
}
