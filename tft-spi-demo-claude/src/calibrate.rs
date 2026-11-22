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
    TouchInput,
};

type BoxResult<T> = Result<T, Box<dyn Error>>;

/// Distance of the corner targets from the screen edges
const TARGET_INSET: u16 = 30;
/// Half-length of the target crosshair arms
const TARGET_ARM: u16 = 10;
/// A calibration is accepted if every target maps back within this many pixels.
const MAX_ERROR_PX: f64 = 8.0;

/// How long "Calibrated" / "try again" messages stay up (none in tests)
#[cfg(not(test))]
const MESSAGE_TIME: Duration = Duration::from_millis(1500);
#[cfg(test)]
const MESSAGE_TIME: Duration = Duration::ZERO;

/// Loads the calibration from `path` if it exists and matches the current
/// rotation; otherwise (or if `force` is set) runs [`run`] and saves the result.
pub fn load_or_calibrate<S: TftSpi, T: TouchInput>(
    display: &mut TftDisplay<S>,
    touch: &mut T,
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
pub fn run<S: TftSpi, T: TouchInput>(display: &mut TftDisplay<S>, touch: &mut T) -> BoxResult<Calibration> {
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
                    thread::sleep(MESSAGE_TIME / 2);
                    return Ok(calibration);
                }
                format!("Off by {error:.0} px, try again")
            }
            None => "Bad readings, try again".to_string(),
        };

        display.fill_screen(Color::BLACK)?;
        draw_centered(display, h / 2 - 8, &message, Color::YELLOW, 2)?;
        thread::sleep(MESSAGE_TIME);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tft_display::mock::MockSpi;
    use std::collections::VecDeque;

    /// Touches played back in order. Each time a touch is requested it also
    /// records where the red target crosshair is on the mock panel.
    struct ScriptedTouch {
        touches: VecDeque<(u16, u16)>,
        panel: MockSpi,
        targets_seen: Vec<Option<(usize, usize)>>,
        releases: usize,
    }

    impl ScriptedTouch {
        fn new(panel: &MockSpi, touches: impl IntoIterator<Item = (u16, u16)>) -> Self {
            Self {
                touches: touches.into_iter().collect(),
                panel: panel.clone(),
                targets_seen: Vec::new(),
                releases: 0,
            }
        }
    }

    impl TouchInput for ScriptedTouch {
        fn wait_for_touch(&mut self) -> BoxResult<(u16, u16)> {
            self.targets_seen.push(red_crosshair(&self.panel));
            self.touches.pop_front().ok_or_else(|| "no more scripted touches".into())
        }

        fn wait_for_release(&mut self) {
            self.releases += 1;
        }
    }

    /// Center of the red crosshair on the panel, or `None` unless there is
    /// exactly one: the row and the column that each hold more than one red pixel.
    fn red_crosshair(panel: &MockSpi) -> Option<(usize, usize)> {
        let p = panel.0.borrow();
        let red = Color::RED.to_rgb666_bytes();
        let is_red = |x: usize, y: usize| p.pixels[y * p.width + x] == red;
        let rows: Vec<usize> = (0..p.height)
            .filter(|&y| (0..p.width).filter(|&x| is_red(x, y)).count() > 1)
            .collect();
        let cols: Vec<usize> = (0..p.width)
            .filter(|&x| (0..p.height).filter(|&y| is_red(x, y)).count() > 1)
            .collect();
        match (cols.as_slice(), rows.as_slice()) {
            (&[x], &[y]) => Some((x, y)),
            _ => None,
        }
    }

    /// Raw reading for a screen point on a simulated touch panel whose axes
    /// are swapped and mirrored relative to the 480x320 landscape screen.
    fn raw_for((x, y): (u16, u16)) -> (u16, u16) {
        let raw_x = 200.0 + f64::from(y) * (3700.0 / 320.0);
        let raw_y = 3900.0 - f64::from(x) * (3600.0 / 480.0);
        (raw_x.round() as u16, raw_y.round() as u16)
    }

    /// The 5 targets on a 480x320 screen, in the order they are shown
    const TARGETS: [(u16, u16); 5] = [(30, 30), (449, 30), (449, 289), (30, 289), (240, 160)];

    fn landscape() -> (TftDisplay<MockSpi>, MockSpi) {
        let spi = MockSpi::new(480, 320);
        let mut display = TftDisplay::new(spi.clone());
        display.set_rotation(Rotate::Degrees90).unwrap();
        (display, spi)
    }

    fn accurate_touches() -> Vec<(u16, u16)> {
        TARGETS.iter().map(|&t| raw_for(t)).collect()
    }

    fn assert_maps_accurately(calibration: &Calibration) {
        for point in TARGETS.into_iter().chain([(100, 250), (400, 60), (0, 0), (479, 319)]) {
            let (x, y) = calibration.to_screen(raw_for(point), 480, 320);
            let error = (f64::from(x) - f64::from(point.0)).hypot(f64::from(y) - f64::from(point.1));
            assert!(error <= 1.5, "{point:?} mapped to ({x}, {y})");
        }
    }

    #[test]
    fn run_shows_targets_in_order_and_fits() {
        let (mut display, spi) = landscape();
        let mut touch = ScriptedTouch::new(&spi, accurate_touches());
        let calibration = run(&mut display, &mut touch).unwrap();

        assert_maps_accurately(&calibration);
        let expected: Vec<_> = TARGETS
            .iter()
            .map(|&(x, y)| Some((usize::from(x), usize::from(y))))
            .collect();
        assert_eq!(touch.targets_seen, expected, "one red target at a time, in order");
        assert_eq!(touch.releases, 5);

        // Ends on the "Calibrated" message: green text, no targets left
        let p = spi.0.borrow();
        assert!(p.pixels.contains(&Color::GREEN.to_rgb666_bytes()));
        assert!(!p.pixels.contains(&Color::RED.to_rgb666_bytes()));
    }

    #[test]
    fn run_retries_after_an_inaccurate_round() {
        let (mut display, spi) = landscape();
        // First round: the center touch lands 60 px right of the target
        let mut first = accurate_touches();
        first[4] = raw_for((300, 160));
        let mut touch = ScriptedTouch::new(&spi, first.into_iter().chain(accurate_touches()));

        let calibration = run(&mut display, &mut touch).unwrap();
        assert_maps_accurately(&calibration);
        assert_eq!(touch.targets_seen.len(), 10, "two full rounds of targets");
        assert!(touch.touches.is_empty());
    }

    #[test]
    fn run_retries_after_unusable_readings() {
        let (mut display, spi) = landscape();
        // First round: every touch reads the same, so no calibration can be fitted
        let same = vec![(2000, 2000); 5];
        let mut touch = ScriptedTouch::new(&spi, same.into_iter().chain(accurate_touches()));

        let calibration = run(&mut display, &mut touch).unwrap();
        assert_maps_accurately(&calibration);
        assert_eq!(touch.targets_seen.len(), 10);
    }

    #[test]
    fn run_passes_on_touch_errors() {
        let (mut display, spi) = landscape();
        let mut touch = ScriptedTouch::new(&spi, accurate_touches().into_iter().take(2));
        let err = run(&mut display, &mut touch).unwrap_err();
        assert_eq!(err.to_string(), "no more scripted touches");
    }

    /// A temp directory for one test, removed when dropped.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("calibrate-{name}-{}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn load_or_calibrate_saves_then_loads() {
        let dir = TempDir::new("save-load");
        let path = dir.0.join("touch_calibration.txt");
        let (mut display, spi) = landscape();

        // No file yet: calibrates and saves
        let mut touch = ScriptedTouch::new(&spi, accurate_touches());
        let calibrated = load_or_calibrate(&mut display, &mut touch, &path, false).unwrap();
        assert_eq!(touch.targets_seen.len(), 5);
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.starts_with("90 "), "file records the rotation: {saved:?}");
        assert_eq!(saved.split_whitespace().count(), 7, "rotation + 6 coefficients: {saved:?}");

        // File present: loads it without asking for touches
        let mut no_touches = ScriptedTouch::new(&spi, []);
        let loaded = load_or_calibrate(&mut display, &mut no_touches, &path, false).unwrap();
        assert_eq!(loaded, calibrated);
        assert!(no_touches.targets_seen.is_empty());
    }

    #[test]
    fn load_or_calibrate_recalibrates_when_forced() {
        let dir = TempDir::new("force");
        let path = dir.0.join("touch_calibration.txt");
        let (mut display, spi) = landscape();
        let touches = accurate_touches().into_iter().chain(accurate_touches());
        let mut touch = ScriptedTouch::new(&spi, touches);

        load_or_calibrate(&mut display, &mut touch, &path, false).unwrap();
        load_or_calibrate(&mut display, &mut touch, &path, true).unwrap();
        assert_eq!(touch.targets_seen.len(), 10, "--calibrate runs it again");
    }

    #[test]
    fn load_or_calibrate_ignores_unusable_files() {
        for (name, contents) in [
            ("other-rotation", "0 1 0 0 0 1 0\n"),
            ("garbage", "not a calibration\n"),
            ("too-few-numbers", "90 1 2 3\n"),
            ("bad-rotation", "45 1 0 0 0 1 0\n"),
            ("empty", ""),
        ] {
            let dir = TempDir::new(name);
            let path = dir.0.join("touch_calibration.txt");
            fs::write(&path, contents).unwrap();

            let (mut display, spi) = landscape();
            let mut touch = ScriptedTouch::new(&spi, accurate_touches());
            let calibration = load_or_calibrate(&mut display, &mut touch, &path, false).unwrap();
            assert_eq!(touch.targets_seen.len(), 5, "{name}: should calibrate");
            assert_maps_accurately(&calibration);
            assert!(fs::read_to_string(&path).unwrap().starts_with("90 "), "{name}: file replaced");
        }
    }

    #[test]
    fn calibration_is_per_rotation() {
        let dir = TempDir::new("rotation");
        let path = dir.0.join("touch_calibration.txt");
        let (mut display, spi) = landscape();
        let mut touch = ScriptedTouch::new(&spi, accurate_touches());
        load_or_calibrate(&mut display, &mut touch, &path, false).unwrap();

        // Same file, display now in portrait: needs a new calibration
        let spi = MockSpi::new(320, 480);
        let mut portrait = TftDisplay::new(spi.clone());
        portrait.set_rotation(Rotate::Degrees0).unwrap();
        let portrait_targets = [(30, 30), (289, 30), (289, 449), (30, 449), (160, 240)];
        let mut touch = ScriptedTouch::new(&spi, portrait_targets.map(|(x, y)| raw_for((y, x))));
        load_or_calibrate(&mut portrait, &mut touch, &path, false).unwrap();

        let expected: Vec<_> = portrait_targets
            .iter()
            .map(|&(x, y)| Some((usize::from(x), usize::from(y))))
            .collect();
        assert_eq!(touch.targets_seen, expected, "portrait targets");
        assert!(fs::read_to_string(&path).unwrap().starts_with("0 "));
    }
}
