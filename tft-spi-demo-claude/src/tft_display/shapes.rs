//! Rasterizes lines and circles into horizontal/vertical runs of pixels.
//!
//! Each run is passed to `emit` as a rectangle `(x, y, w, h)` with `w == 1` or
//! `h == 1` (filled circles also produce wider blocks). Sending runs instead of
//! single pixels means far fewer SPI transfers. Coordinates are signed and may
//! be off-screen; the caller clips.

/// `(x, y, width, height)`
pub type Rect = (i32, i32, i32, i32);

/// Bresenham line from (x0, y0) to (x1, y1), both ends included.
pub fn line<E>(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    mut emit: impl FnMut(Rect) -> Result<(), E>,
) -> Result<(), E> {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    // Mostly-horizontal lines are emitted as horizontal runs, steep ones as vertical runs.
    let horizontal = dx >= -dy;

    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    let mut run_start = (x0, y0);

    loop {
        if x == x1 && y == y1 {
            return emit(run(run_start, (x, y)));
        }

        let (mut next_x, mut next_y) = (x, y);
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            next_x += sx;
        }
        if e2 <= dx {
            err += dx;
            next_y += sy;
        }

        let leaves_run = if horizontal { next_y != y } else { next_x != x };
        if leaves_run {
            emit(run(run_start, (x, y)))?;
            run_start = (next_x, next_y);
        }
        (x, y) = (next_x, next_y);
    }
}

/// 1-pixel rectangle outline with its top-left corner at (x, y). Each pixel is
/// drawn once: top and bottom edges span the full width, the sides fill in
/// between them.
pub fn rectangle<E>(
    x: i32,
    y: i32,
    w: u16,
    h: u16,
    mut emit: impl FnMut(Rect) -> Result<(), E>,
) -> Result<(), E> {
    let (w, h) = (i32::from(w), i32::from(h));
    if w == 0 || h == 0 {
        return Ok(());
    }
    if w <= 2 || h <= 2 {
        // No inside to leave empty: it's a solid block
        return emit((x, y, w, h));
    }

    emit((x, y, w, 1))?; // top
    emit((x, y + h - 1, w, 1))?; // bottom
    emit((x, y + 1, 1, h - 2))?; // left
    emit((x + w - 1, y + 1, 1, h - 2)) // right
}

/// Circle outline centered on (cx, cy) (midpoint algorithm).
pub fn circle<E>(
    cx: i32,
    cy: i32,
    radius: u16,
    mut emit: impl FnMut(Rect) -> Result<(), E>,
) -> Result<(), E> {
    if radius == 0 {
        return emit((cx, cy, 1, 1));
    }
    octant_runs(radius, |x_start, x_end, y| {
        let len = x_end - x_start + 1;
        if x_start == 0 {
            // Runs touching the axes: one span across the axis instead of two.
            let width = 2 * x_end + 1;
            emit((cx - x_end, cy - y, width, 1))?; // top
            emit((cx - x_end, cy + y, width, 1))?; // bottom
            emit((cx - y, cy - x_end, 1, width))?; // left
            emit((cx + y, cy - x_end, 1, width))?; // right
        } else {
            for sy in [-1, 1] {
                // top / bottom, both sides
                emit((cx + x_start, cy + sy * y, len, 1))?;
                emit((cx - x_end, cy + sy * y, len, 1))?;
            }
            for sx in [-1, 1] {
                // left / right, both halves
                emit((cx + sx * y, cy + x_start, 1, len))?;
                emit((cx + sx * y, cy - x_end, 1, len))?;
            }
        }
        Ok(())
    })
}

/// Filled circle centered on (cx, cy), covering the same pixels as [`circle`]
/// plus everything inside it.
pub fn filled_circle<E>(
    cx: i32,
    cy: i32,
    radius: u16,
    emit: impl FnMut(Rect) -> Result<(), E>,
) -> Result<(), E> {
    let size = 2 * radius + 1;
    filled_round_rectangle(cx - i32::from(radius), cy - i32::from(radius), size, size, radius, emit)
}

/// Filled rectangle with its top-left corner at (x, y) and corners rounded
/// with `radius`. The radius is reduced to fit if it's more than half the
/// shorter side, so a large radius gives a pill or circle shape.
///
/// Each corner is a quarter of a [`filled_circle`], so the curves match
/// [`circle`] pixel for pixel.
pub fn filled_round_rectangle<E>(
    x: i32,
    y: i32,
    w: u16,
    h: u16,
    radius: u16,
    mut emit: impl FnMut(Rect) -> Result<(), E>,
) -> Result<(), E> {
    if w == 0 || h == 0 {
        return Ok(());
    }
    let radius = radius.min((w - 1) / 2).min((h - 1) / 2);
    let (w, h, r) = (i32::from(w), i32::from(h), i32::from(radius));
    if r == 0 {
        return emit((x, y, w, h));
    }

    // Centers of the corner circles
    let (left, right) = (x + r, x + w - 1 - r);
    let (top, bottom) = (y + r, y + h - 1 - r);
    let stretch = right - left; // extra width between the left and right corners

    octant_runs(radius, |x_start, x_end, dy| {
        let len = x_end - x_start + 1;
        // Rows top - dy / bottom + dy: from the left corner's edge to the right corner's
        emit((left - x_end, top - dy, stretch + 2 * x_end + 1, 1))?;
        emit((left - x_end, bottom + dy, stretch + 2 * x_end + 1, 1))?;
        // Rows top - x / bottom + x for x in x_start..=x_end: span left - dy ..= right + dy
        emit((left - dy, top - x_end, stretch + 2 * dy + 1, len))?;
        emit((left - dy, bottom + x_start, stretch + 2 * dy + 1, len))?;
        Ok(())
    })?;

    // Full-width block between the top and bottom corner rows
    if bottom - top > 1 {
        emit((x, top + 1, w, bottom - top - 1))?;
    }
    Ok(())
}

/// Walks the octant from 12 o'clock to 45° (x from 0 up to y) and reports each
/// run of consecutive x values that share the same y as `(x_start, x_end, y)`.
fn octant_runs<E>(
    radius: u16,
    mut run: impl FnMut(i32, i32, i32) -> Result<(), E>,
) -> Result<(), E> {
    let r = i32::from(radius);
    let (mut x, mut y) = (0, r);
    let mut d = 1 - r;
    let mut run_start = 0;

    while x <= y {
        let (cur_x, cur_y) = (x, y);
        if d < 0 {
            d += 2 * x + 3;
        } else {
            d += 2 * (x - y) + 5;
            y -= 1;
        }
        x += 1;

        if y != cur_y || x > y {
            run(run_start, cur_x, cur_y)?;
            run_start = x;
        }
    }
    Ok(())
}

/// Rectangle covering a run between two points on the same row or column.
fn run((ax, ay): (i32, i32), (bx, by): (i32, i32)) -> Rect {
    (ax.min(bx), ay.min(by), (bx - ax).abs() + 1, (by - ay).abs() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    type Pixels = BTreeSet<(i32, i32)>;

    fn rasterize(draw: impl FnOnce(&mut dyn FnMut(Rect) -> Result<(), ()>)) -> (Pixels, usize) {
        let mut pixels = Pixels::new();
        let mut rects = 0;
        draw(&mut |(x, y, w, h)| {
            assert!(w > 0 && h > 0, "empty rect {:?}", (x, y, w, h));
            rects += 1;
            for px in x..x + w {
                for py in y..y + h {
                    pixels.insert((px, py));
                }
            }
            Ok(())
        });
        (pixels, rects)
    }

    fn line_pixels(x0: i32, y0: i32, x1: i32, y1: i32) -> (Pixels, usize) {
        rasterize(|emit| line(x0, y0, x1, y1, emit).unwrap())
    }

    /// Plain per-pixel Bresenham to compare against.
    fn reference_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Pixels {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        let mut pixels = Pixels::new();
        loop {
            pixels.insert((x, y));
            if x == x1 && y == y1 {
                return pixels;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Plain per-pixel midpoint circle (8-way symmetry) to compare against.
    fn reference_circle(cx: i32, cy: i32, r: i32) -> Pixels {
        let (mut x, mut y, mut d) = (0, r, 1 - r);
        let mut pixels = Pixels::new();
        while x <= y {
            for (px, py) in [(x, y), (y, x)] {
                for (sx, sy) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                    pixels.insert((cx + sx * px, cy + sy * py));
                }
            }
            if d < 0 {
                d += 2 * x + 3;
            } else {
                d += 2 * (x - y) + 5;
                y -= 1;
            }
            x += 1;
        }
        pixels
    }

    #[test]
    fn lines_match_per_pixel_bresenham() {
        let ends = [
            (0, 0, 10, 0),
            (0, 0, 0, 10),
            (0, 0, 10, 10),
            (0, 0, 10, 3),
            (0, 0, 3, 10),
            (10, 3, 0, 0),
            (5, 5, -7, 2),
            (5, 5, 2, -7),
            (-20, 15, 479, 300),
            (3, 3, 3, 3),
        ];
        for (x0, y0, x1, y1) in ends {
            let (pixels, _) = line_pixels(x0, y0, x1, y1);
            assert_eq!(pixels, reference_line(x0, y0, x1, y1), "line {:?}", (x0, y0, x1, y1));
        }
    }

    #[test]
    fn straight_lines_are_one_rect() {
        assert_eq!(line_pixels(0, 5, 100, 5).1, 1);
        assert_eq!(line_pixels(7, 100, 7, 0).1, 1);
        // A shallow line is a handful of runs, not one rect per pixel
        let (pixels, rects) = line_pixels(0, 0, 200, 10);
        assert_eq!(pixels.len(), 201);
        assert_eq!(rects, 11);
    }

    #[test]
    fn rectangle_outlines() {
        let outline = |x: i32, y: i32, w: u16, h: u16| {
            rasterize(|emit| {
                let mut seen = Pixels::new();
                rectangle(x, y, w, h, |(rx, ry, rw, rh)| {
                    // No pixel is drawn twice
                    for px in rx..rx + rw {
                        for py in ry..ry + rh {
                            assert!(seen.insert((px, py)), "({px}, {py}) drawn twice");
                        }
                    }
                    emit((rx, ry, rw, rh))
                })
                .unwrap()
            })
        };
        let border = |x: i32, y: i32, w: i32, h: i32| -> Pixels {
            (x..x + w)
                .flat_map(|px| (y..y + h).map(move |py| (px, py)))
                .filter(|&(px, py)| px == x || px == x + w - 1 || py == y || py == y + h - 1)
                .collect()
        };

        for (x, y, w, h) in [(0, 0, 10, 5), (-3, 7, 4, 3), (5, 5, 3, 3), (2, 2, 1, 1), (0, 0, 2, 9), (0, 0, 9, 1)] {
            let (pixels, rects) = outline(x, y, w, h);
            assert_eq!(pixels, border(x, y, i32::from(w), i32::from(h)), "{:?}", (x, y, w, h));
            assert!(rects <= 4);
        }
        assert_eq!(outline(5, 5, 0, 10).0.len(), 0);
        assert_eq!(outline(5, 5, 10, 0).0.len(), 0);
    }

    #[test]
    fn circles_match_per_pixel_midpoint() {
        for r in [0u16, 1, 2, 3, 5, 10, 37, 100, 159] {
            let (pixels, rects) = rasterize(|emit| circle(50, 40, r, emit).unwrap());
            let expected = reference_circle(50, 40, i32::from(r));
            assert_eq!(pixels, expected, "radius {r}");
            assert!(rects <= expected.len(), "radius {r}: {rects} rects");
            if r >= 10 {
                // Runs should cut the number of transfers well below one per pixel
                assert!(rects * 2 <= expected.len(), "radius {r}: {rects} rects for {} px", expected.len());
            }
        }
    }

    #[test]
    fn filled_circle_covers_outline_and_inside() {
        for r in [0u16, 1, 2, 5, 10, 37, 100] {
            let (filled, _) = rasterize(|emit| filled_circle(0, 0, r, emit).unwrap());
            let outline = reference_circle(0, 0, i32::from(r));
            assert!(outline.is_subset(&filled), "radius {r}: outline not covered");

            // Each row is one solid span bounded by the outline
            let r = i32::from(r);
            for y in -r..=r {
                let row: Vec<i32> = filled.iter().filter(|p| p.1 == y).map(|p| p.0).collect();
                let outline_row: Vec<i32> = outline.iter().filter(|p| p.1 == y).map(|p| p.0).collect();
                let (min, max) = (row[0], *row.last().unwrap());
                assert_eq!(row.len() as i32, max - min + 1, "radius {r} row {y} has gaps");
                assert_eq!(min, *outline_row.iter().min().unwrap(), "radius {r} row {y}");
                assert_eq!(max, *outline_row.iter().max().unwrap(), "radius {r} row {y}");
            }
            assert!(filled.iter().all(|p| p.1.abs() <= r), "radius {r}: rows outside circle");
        }
    }

    /// Expected rounded rectangle: a plus-shaped cross of two rectangles, and a
    /// filled quarter of the reference circle in each corner.
    fn reference_round_rectangle(x: i32, y: i32, w: i32, h: i32, r: i32) -> Pixels {
        let (left, right, top, bottom) = (x + r, x + w - 1 - r, y + r, y + h - 1 - r);
        let mut pixels = Pixels::new();
        for px in x..x + w {
            for py in y..y + h {
                if (left..=right).contains(&px) || (top..=bottom).contains(&py) {
                    pixels.insert((px, py));
                }
            }
        }
        let outline = reference_circle(0, 0, r);
        for (cx, cy, sx, sy) in [(left, top, -1, -1), (right, top, 1, -1), (left, bottom, -1, 1), (right, bottom, 1, 1)] {
            for dy in 0..=r {
                // Widest outline pixel on this row of the circle
                let reach = outline.iter().filter(|p| p.1 == dy).map(|p| p.0).max().unwrap();
                for dx in 0..=reach {
                    pixels.insert((cx + sx * dx, cy + sy * dy));
                }
            }
        }
        pixels
    }

    #[test]
    fn filled_round_rectangles() {
        let cases = [
            (0, 0, 100, 40, 10, 10),
            (5, -3, 37, 22, 7, 7),
            (0, 0, 60, 20, 50, 9),  // radius reduced to fit: (20 - 1) / 2
            (0, 0, 21, 21, 10, 10), // radius = half: a circle
            (0, 0, 50, 21, 10, 10), // pill shape
            (0, 0, 20, 30, 3, 3),
            (0, 0, 2, 2, 5, 0),
            (-10, -10, 15, 8, 0, 0),
        ];
        for (x, y, w, h, radius, effective) in cases {
            let (pixels, rects) = rasterize(|emit| filled_round_rectangle(x, y, w, h, radius, emit).unwrap());
            let expected = reference_round_rectangle(x, y, i32::from(w), i32::from(h), effective);
            assert_eq!(pixels, expected, "{:?}", (x, y, w, h, radius));
            assert!(rects <= usize::from(h), "{:?}: {rects} rects for {h} rows", (x, y, w, h, radius));
        }
        assert!(rasterize(|emit| filled_round_rectangle(0, 0, 0, 10, 3, emit).unwrap()).0.is_empty());
        assert!(rasterize(|emit| filled_round_rectangle(0, 0, 10, 0, 3, emit).unwrap()).0.is_empty());
    }
}
