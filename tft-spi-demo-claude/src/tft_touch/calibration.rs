//! Maps raw XPT2046 readings (0..=4095) to screen coordinates.
//!
//! Uses an affine transform fitted by least squares from 3 or more
//! (raw, screen) point pairs:
//!
//! ```text
//! screen_x = x[0] * raw_x + x[1] * raw_y + x[2]
//! screen_y = y[0] * raw_x + y[1] * raw_y + y[2]
//! ```
//!
//! This handles swapped axes, mirroring, scale, offset and slight skew in one
//! step, so it doesn't matter how the touch panel is wired relative to the LCD.
//! A calibration is only valid for the display rotation it was taken in.

/// A calibration point: the raw touch reading and the screen point it should map to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationPoint {
    pub raw: (u16, u16),
    pub screen: (u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Calibration {
    /// Coefficients for screen x: `x[0] * raw_x + x[1] * raw_y + x[2]`
    pub x: [f64; 3],
    /// Coefficients for screen y: `y[0] * raw_x + y[1] * raw_y + y[2]`
    pub y: [f64; 3],
}

impl Calibration {
    /// Least-squares fit from 3 or more points. Returns `None` if there are
    /// fewer than 3 points or they are (nearly) in a straight line.
    pub fn fit(points: &[CalibrationPoint]) -> Option<Self> {
        if points.len() < 3 {
            return None;
        }

        // Normal equations: (Aᵀ A) c = Aᵀ s, with rows of A = [raw_x, raw_y, 1]
        let mut ata = [[0.0f64; 3]; 3];
        let mut atx = [0.0f64; 3];
        let mut aty = [0.0f64; 3];
        for p in points {
            let row = [f64::from(p.raw.0), f64::from(p.raw.1), 1.0];
            for i in 0..3 {
                for j in 0..3 {
                    ata[i][j] += row[i] * row[j];
                }
                atx[i] += row[i] * f64::from(p.screen.0);
                aty[i] += row[i] * f64::from(p.screen.1);
            }
        }

        Some(Self {
            x: solve3(ata, atx)?,
            y: solve3(ata, aty)?,
        })
    }

    /// Maps a raw reading to (unclamped) screen coordinates.
    pub fn map(&self, raw: (u16, u16)) -> (f64, f64) {
        let (rx, ry) = (f64::from(raw.0), f64::from(raw.1));
        (
            self.x[0] * rx + self.x[1] * ry + self.x[2],
            self.y[0] * rx + self.y[1] * ry + self.y[2],
        )
    }

    /// Maps a raw reading to screen coordinates, clamped to a `width` x `height` screen.
    pub fn to_screen(&self, raw: (u16, u16), width: u16, height: u16) -> (u16, u16) {
        let (x, y) = self.map(raw);
        let clamp = |v: f64, size: u16| v.round().clamp(0.0, f64::from(size.saturating_sub(1))) as u16;
        (clamp(x, width), clamp(y, height))
    }

    /// Largest distance in pixels between where a point should be and where it maps.
    pub fn max_error(&self, points: &[CalibrationPoint]) -> f64 {
        points
            .iter()
            .map(|p| {
                let (x, y) = self.map(p.raw);
                (x - f64::from(p.screen.0)).hypot(y - f64::from(p.screen.1))
            })
            .fold(0.0, f64::max)
    }

    /// Text form: six space-separated coefficients.
    pub fn to_text(&self) -> String {
        let [a, b, c] = self.x;
        let [d, e, f] = self.y;
        format!("{a} {b} {c} {d} {e} {f}")
    }

    /// Parses the output of [`Calibration::to_text`].
    pub fn from_text(text: &str) -> Option<Self> {
        let values: Vec<f64> = text
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        match values[..] {
            [a, b, c, d, e, f] if values.iter().all(|v| v.is_finite()) => Some(Self {
                x: [a, b, c],
                y: [d, e, f],
            }),
            _ => None,
        }
    }
}

/// Solves the 3x3 system `m * c = v` with Cramer's rule.
fn solve3(m: [[f64; 3]; 3], v: [f64; 3]) -> Option<[f64; 3]> {
    let det = det3(m);
    let scale = m[0][0] * m[1][1] * m[2][2];
    if !det.is_finite() || det.abs() <= 1e-9 * scale.abs() {
        return None;
    }

    let mut result = [0.0; 3];
    for (col, out) in result.iter_mut().enumerate() {
        let mut mc = m;
        for row in 0..3 {
            mc[row][col] = v[row];
        }
        *out = det3(mc) / det;
    }
    Some(result)
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simulates a touch panel whose axes are swapped and mirrored relative to
    /// a 480x320 landscape screen: raw_x follows screen y, raw_y follows
    /// (inverted) screen x.
    fn simulated_raw(screen: (u16, u16)) -> (u16, u16) {
        let (sx, sy) = (f64::from(screen.0), f64::from(screen.1));
        let raw_x = 200.0 + sy * (3700.0 / 320.0);
        let raw_y = 3900.0 - sx * (3600.0 / 480.0);
        (raw_x.round() as u16, raw_y.round() as u16)
    }

    fn points(targets: &[(u16, u16)]) -> Vec<CalibrationPoint> {
        targets
            .iter()
            .map(|&screen| CalibrationPoint {
                raw: simulated_raw(screen),
                screen,
            })
            .collect()
    }

    #[test]
    fn fits_swapped_and_mirrored_axes() {
        let pts = points(&[(30, 30), (450, 30), (450, 290), (30, 290), (240, 160)]);
        let cal = Calibration::fit(&pts).expect("fit");

        assert!(cal.max_error(&pts) < 1.0, "max error {}", cal.max_error(&pts));
        // A point that wasn't used for the fit
        assert_eq!(cal.to_screen(simulated_raw((100, 250)), 480, 320), (100, 250));
    }

    #[test]
    fn clamps_to_screen() {
        let pts = points(&[(30, 30), (450, 30), (450, 290), (30, 290)]);
        let cal = Calibration::fit(&pts).unwrap();
        // Raw readings past the panel edges
        assert_eq!(cal.to_screen((0, 4095), 480, 320), (0, 0));
        assert_eq!(cal.to_screen((4095, 0), 480, 320), (479, 319));
    }

    #[test]
    fn rejects_too_few_or_collinear_points() {
        assert!(Calibration::fit(&points(&[(30, 30), (450, 290)])).is_none());
        assert!(Calibration::fit(&points(&[(30, 30), (240, 160), (450, 290)])).is_none());
    }

    #[test]
    fn text_round_trip() {
        let pts = points(&[(30, 30), (450, 30), (450, 290), (30, 290)]);
        let cal = Calibration::fit(&pts).unwrap();
        assert_eq!(Calibration::from_text(&cal.to_text()), Some(cal));
        assert_eq!(Calibration::from_text("1 2 3"), None);
        assert_eq!(Calibration::from_text("1 2 3 4 5 x"), None);
        assert_eq!(Calibration::from_text("1 2 3 4 5 NaN"), None);
    }
}
