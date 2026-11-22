//! The demo app: shapes, text and touch drawing, or a picture viewer.
//! Raspberry Pi (Linux) only.

use std::{
    env,
    error::Error,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use crate::calibrate;
use crate::picture::{self, Scale};
use crate::tft_display::{
    color::Color,
    enums::Rotate,
    tft_display::{Gamma, TftDisplay},
};
use crate::tft_spi::TftSpiImpl;
use crate::tft_touch::tft_touch::TftTouch;
use crate::ui::{self, BUTTONS};

type Display = TftDisplay<TftSpiImpl>;

/// Saved in the current directory. Run with `--calibrate` to redo it.
const CALIBRATION_FILE: &str = "touch_calibration.txt";

/// Usage:
///   tft-spi-demo-claude [--calibrate]      demo: shapes, text, touch drawing
///   tft-spi-demo-claude [--fill] <image>   show a JPEG or PNG and exit
///   tft-spi-demo-claude --colors           color gradients and exit
///
/// `TFT_GAMMA=full|analog|none` picks the gamma tables sent at startup.
pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let has_flag = |flag: &str| args.iter().any(|arg| arg == flag);
    let image_path = args.iter().find(|arg| !arg.starts_with("--"));

    // Display on CE0 (held low as a GPIO), touch on CE1: separate SPI handles.
    let spi = TftSpiImpl::new()?;
    let mut display = TftDisplay::new(spi.clone());
    let gamma = match env::var("TFT_GAMMA") {
        Ok(name) => Gamma::from_name(&name).unwrap_or_else(|| {
            eprintln!("ignoring TFT_GAMMA={name:?}: use full, analog or none");
            Gamma::PanelDefault
        }),
        Err(_) => Gamma::PanelDefault,
    };
    println!("gamma: {gamma:?} (set TFT_GAMMA=full, analog or none to change)");
    display.set_gamma(gamma);
    display.initialize()?;

    if has_flag("--colors") {
        return color_test(&mut display);
    }

    if let Some(path) = image_path {
        let scale = if has_flag("--fill") { Scale::Fill } else { Scale::Fit };
        return show_image(&mut display, Path::new(path), scale);
    }

    let force_calibration = has_flag("--calibrate");
    let mut touch = TftTouch::new(spi)?;

    rotation_test(&mut display)?;
    shapes_test(&mut display)?;

    // Landscape UI. The calibration only applies to the rotation it was made in.
    display.set_rotation(Rotate::Degrees90)?;
    let calibration = calibrate::load_or_calibrate(
        &mut display,
        &mut touch,
        Path::new(CALIBRATION_FILE),
        force_calibration,
    )?;

    ui::draw_ui(&mut display)?;

    // Drag to draw; tap a button to highlight it. Ctrl-C to quit.
    println!("touch the screen (Ctrl-C to quit)...");
    let mut last_point: Option<(i32, i32)> = None;
    loop {
        let Some(raw) = touch.read_filtered(TftTouch::FILTER_SAMPLES)? else {
            last_point = None; // finger lifted: next touch starts a new stroke
            thread::sleep(Duration::from_millis(10));
            continue;
        };
        let (x, y) = calibration.to_screen(raw, display.width(), display.height());

        if let Some(index) = ui::button_at(x, y, display.height()) {
            last_point = None;
            println!("touch ({x}, {y}): button {}", BUTTONS[index].0);
            ui::draw_button(&mut display, index, true)?;
            touch.wait_for_release();
            ui::draw_button(&mut display, index, false)?;
        } else {
            println!("touch ({x}, {y}) raw {raw:?}");
            let point = (i32::from(x), i32::from(y));
            match last_point {
                Some((last_x, last_y)) => {
                    display.draw_line(last_x, last_y, point.0, point.1, Color::WHITE)?
                }
                None => display.fill_circle(point.0, point.1, 1, Color::WHITE)?,
            }
            last_point = Some(point);
        }
    }
}

/// Landscape screen of gradients, dark (left) to bright (right): red, green,
/// blue, grey, and orange to beige (the colors of the sample photo). Each
/// band should brighten smoothly and keep its hue.
fn color_test(display: &mut Display) -> Result<(), Box<dyn Error>> {
    display.set_rotation(Rotate::Degrees90)?;
    let (w, h) = (usize::from(display.width()), usize::from(display.height()));
    let bands: [(&str, [u8; 3]); 6] = [
        ("red", [255, 0, 0]),
        ("green", [0, 255, 0]),
        ("blue", [0, 0, 255]),
        ("grey", [255, 255, 255]),
        ("orange", [255, 128, 32]),
        ("beige", [240, 210, 180]),
    ];
    let band_h = h / bands.len();
    let mut rgb = vec![0u8; w * h * 3];
    for y in 0..h {
        let (_, full) = bands[(y / band_h).min(bands.len() - 1)];
        for x in 0..w {
            for (c, &value) in full.iter().enumerate() {
                rgb[(y * w + x) * 3 + c] = (usize::from(value) * x / (w - 1)) as u8;
            }
        }
    }
    display.draw_image(0, 0, w as u16, h as u16, &rgb)?;
    for (i, (name, _)) in bands.iter().enumerate() {
        let y = (i * band_h + band_h / 2 - 4) as u16;
        display.draw_text(4, y, name, Color::WHITE, Color::BLACK, 1)?;
    }
    Ok(())
}

/// Lines and circles, including some that run off the screen edges.
fn shapes_test(display: &mut Display) -> Result<(), Box<dyn Error>> {
    display.set_rotation(Rotate::Degrees90)?;
    display.fill_screen(Color::BLACK)?;
    let (w, h) = (i32::from(display.width()), i32::from(display.height()));

    // Fan of lines from the top-left corner to points along the bottom and right edges
    for i in 0..=8 {
        display.draw_line(0, 0, i * (w - 1) / 8, h - 1, Color::CYAN)?;
        display.draw_line(0, 0, w - 1, i * (h - 1) / 8, Color::GREEN)?;
    }

    // Concentric outlines, a filled circle, and circles clipped by the edges
    for r in (10..=60).step_by(10) {
        display.draw_circle(w * 3 / 4, h / 2, r, Color::YELLOW)?;
    }
    display.fill_circle(w * 3 / 4, h / 2, 6, Color::RED)?;
    display.fill_circle(w - 1, h - 1, 40, Color::MAGENTA)?;
    display.draw_circle(-10, h / 2, 50, Color::WHITE)?;

    // Nested rectangle outlines, and one clipped by the top edge
    for i in 0..5 {
        let inset = i * 8;
        let size = (100 - 2 * inset) as u16;
        display.draw_rectangle(w / 4 - 50 + inset, h / 2 - 50 + inset, size, size, Color::RED)?;
    }
    display.draw_rectangle(w / 2 - 30, -20, 60, 50, Color::BLUE)?;

    // Rounded rectangles: small corners, a pill (radius larger than half the
    // height), and one clipped by the left edge
    display.fill_round_rectangle(w / 2 - 60, h / 2 + 40, 120, 40, 6, Color::GREEN)?;
    display.fill_round_rectangle(w / 2 - 60, h / 2 + 90, 120, 30, 100, Color::BROWN)?;
    display.fill_round_rectangle(-20, 10, 80, 50, 15, Color::TAN)?;

    let text_y = display.height() - 20;
    display.draw_text(10, text_y, "lines, circles, rectangles", Color::WHITE, Color::BLACK, 2)?;
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

/// Shows a picture centered on the screen, rotating the screen to portrait
/// for portrait pictures. Fit leaves black borders; Fill crops to cover.
fn show_image(display: &mut Display, path: &Path, scale: Scale) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let image = picture::open(path).map_err(|err| format!("{}: {err}", path.display()))?;
    println!("{}: {}x{}, decoded in {:?}", path.display(), image.width(), image.height(), start.elapsed());

    let shown = picture::show(display, &image, scale)?;
    println!(
        "scaled to {}x{} ({scale:?}) in {:?}, drawn at ({}, {}) in {:?}",
        shown.width, shown.height, shown.scale_time, shown.x, shown.y, shown.draw_time
    );
    Ok(())
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
        // Upright, readable text means the rotation is right; mirrored or
        // sideways text shows which way it's wrong.
        let label = format!("{} deg
{}x{}", rotation.degrees(), display.width(), display.height());
        display.draw_text(20, 20, &label, Color::BLACK, background, 4)?;
        thread::sleep(Duration::from_secs(3));
    }
    Ok(())
}
