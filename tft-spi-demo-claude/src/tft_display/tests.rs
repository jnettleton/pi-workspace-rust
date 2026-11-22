//! Tests for `TftDisplay` against the mock ILI9486 in `mock.rs`, which paints
//! what the display sends into a framebuffer so tests can check the screen.
//! Runs on any platform with `cargo test`.

use std::time::Duration;

use super::{
    color::Color,
    enums::{Command, Rotate},
    error::Error,
    font,
    mock::{MockSpi, GRAM_H, GRAM_W, MADCTL},
    shapes,
    tft_display::TftDisplay,
};

/// A 480x320 landscape display on a mock panel.
fn landscape() -> (TftDisplay<MockSpi>, MockSpi) {
    let spi = MockSpi::new(480, 320);
    let mut display = TftDisplay::new(spi.clone());
    display.set_rotation(Rotate::Degrees90).unwrap();
    (display, spi)
}

/// Test image with (nearly) every pixel different: R from x, G from y, B from both.
fn gradient(w: u16, h: u16) -> Vec<u8> {
    let mut rgb = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rgb.extend_from_slice(&[(x * 4) as u8, (y * 4) as u8, ((x ^ y) * 4) as u8]);
        }
    }
    rgb
}

/// Source pixel as the panel should receive it (RGB666: low 2 bits cleared).
fn src_pixel(rgb: &[u8], w: u16, x: usize, y: usize) -> [u8; 3] {
    let i = (y * usize::from(w) + x) * 3;
    [rgb[i] & 0xFC, rgb[i + 1] & 0xFC, rgb[i + 2] & 0xFC]
}

/// Checks the panel against an image drawn at (ox, oy): every on-screen
/// image pixel matches, the rest is untouched (black), and nothing was sent
/// past the window. Returns the number of visible pixels.
fn check_placement(spi: &MockSpi, rgb: &[u8], w: u16, h: u16, ox: i32, oy: i32) -> usize {
    let p = spi.0.borrow();
    let mut visible = 0;
    for y in 0..p.height {
        for x in 0..p.width {
            let (ix, iy) = (x as i32 - ox, y as i32 - oy);
            let expected = if ix >= 0 && iy >= 0 && ix < i32::from(w) && iy < i32::from(h) {
                visible += 1;
                src_pixel(rgb, w, ix as usize, iy as usize)
            } else {
                [0, 0, 0]
            };
            assert_eq!(p.pixels[y * p.width + x], expected, "screen ({x}, {y}), image at ({ox}, {oy})");
        }
    }
    assert_eq!(p.overflow, 0, "pixels sent past the window");
    assert_eq!(p.pixels_written, visible, "pixels sent vs visible");
    visible
}

#[test]
fn draw_image_on_screen_and_clipped() {
    let (w, h) = (50, 40);
    let rgb = gradient(w, h);
    // On screen, clipped left/top, clipped right/bottom, clipped by one pixel
    for (ox, oy, visible) in [(100, 50, 50 * 40), (-10, -5, 40 * 35), (460, 300, 20 * 20), (-1, -1, 49 * 39)] {
        let (mut display, spi) = landscape();
        display.draw_image(ox, oy, w, h, &rgb).unwrap();
        assert_eq!(check_placement(&spi, &rgb, w, h, ox, oy), visible, "image at ({ox}, {oy})");
    }

    // Bigger than the screen on every side
    let (bw, bh) = (600, 400);
    let big = gradient(bw, bh);
    let (mut display, spi) = landscape();
    display.draw_image(-60, -40, bw, bh, &big).unwrap();
    assert_eq!(check_placement(&spi, &big, bw, bh, -60, -40), 480 * 320);
}

#[test]
fn draw_image_off_screen_sends_nothing() {
    let rgb = gradient(50, 40);
    for (ox, oy) in [(480, 0), (0, 320), (-50, 0), (0, -40), (1000, 1000)] {
        let (mut display, spi) = landscape();
        display.draw_image(ox, oy, 50, 40, &rgb).unwrap();
        assert!(spi.0.borrow().pixel_transfers.is_empty(), "image at ({ox}, {oy})");
    }
}

#[test]
fn draw_image_rejects_wrong_buffer_size() {
    let (mut display, spi) = landscape();
    let err = display.draw_image(0, 0, 50, 40, &[0; 50 * 40 * 3 - 1]).unwrap_err();
    assert!(matches!(err, Error::ImageSize { given: 5999, expected: 6000 }), "{err}");
    assert!(spi.0.borrow().pixel_transfers.is_empty());
}

#[test]
fn full_screen_image_transfers() {
    let rgb = gradient(480, 320);
    let (mut display, spi) = landscape();
    display.draw_image(0, 0, 480, 320, &rgb).unwrap();
    check_placement(&spi, &rgb, 480, 320, 0, 0);

    // 460800 bytes in pixel-aligned 4095-byte transfers
    let transfers = &spi.0.borrow().pixel_transfers;
    assert_eq!(transfers.len(), 113);
    assert!(transfers[..112].iter().all(|&n| n == 4095));
    assert_eq!(transfers.iter().sum::<usize>(), 480 * 320 * 3);
}

const BLACK: [u8; 3] = [0, 0, 0];

/// Checks every panel pixel against `expected(x, y)` and that nothing was
/// sent past an address window. Returns the number of pixels sent.
fn check_frame(spi: &MockSpi, expected: impl Fn(usize, usize) -> [u8; 3]) -> usize {
    let p = spi.0.borrow();
    for y in 0..p.height {
        for x in 0..p.width {
            assert_eq!(p.pixels[y * p.width + x], expected(x, y), "pixel ({x}, {y})");
        }
    }
    assert_eq!(p.overflow, 0, "pixels sent past the window");
    p.pixels_written
}

/// Whether (x, y) is inside the w x h rectangle at (rx, ry).
fn inside(x: usize, y: usize, rx: usize, ry: usize, w: usize, h: usize) -> bool {
    (rx..rx + w).contains(&x) && (ry..ry + h).contains(&y)
}

// ---- fill_rectangle ----

#[test]
fn fill_rectangle_on_screen() {
    let (mut display, spi) = landscape();
    display.fill_rectangle(10, 20, 30, 15, Color::RED).unwrap();

    let red = Color::RED.to_rgb666_bytes();
    let sent = check_frame(&spi, |x, y| if inside(x, y, 10, 20, 30, 15) { red } else { BLACK });
    assert_eq!(sent, 30 * 15);
}

#[test]
fn fill_rectangle_clipped_at_right_and_bottom() {
    let (mut display, spi) = landscape();
    display.fill_rectangle(470, 310, 50, 50, Color::GREEN).unwrap();

    let green = Color::GREEN.to_rgb666_bytes();
    let sent = check_frame(&spi, |x, y| if inside(x, y, 470, 310, 10, 10) { green } else { BLACK });
    assert_eq!(sent, 10 * 10);
}

#[test]
fn fill_rectangle_empty_or_off_screen_sends_nothing() {
    for (x, y, w, h) in [(10, 10, 0, 5), (10, 10, 5, 0), (480, 0, 10, 10), (0, 320, 10, 10), (u16::MAX, 5, 1, 1)] {
        let (mut display, spi) = landscape();
        display.fill_rectangle(x, y, w, h, Color::WHITE).unwrap();
        assert!(spi.0.borrow().pixel_transfers.is_empty(), "rect {:?}", (x, y, w, h));
    }
}

#[test]
fn fill_screen_in_each_rotation() {
    for (rotation, w, h) in [
        (Rotate::Degrees0, 320, 480),
        (Rotate::Degrees90, 480, 320),
        (Rotate::Degrees180, 320, 480),
        (Rotate::Degrees270, 480, 320),
    ] {
        // The mock ignores MADCTL, so it just needs to match the logical size
        let spi = MockSpi::new(w, h);
        let mut display = TftDisplay::new(spi.clone());
        display.set_rotation(rotation).unwrap();
        assert_eq!((usize::from(display.width()), usize::from(display.height())), (w, h));

        display.fill_screen(Color::BLUE).unwrap();
        let blue = Color::BLUE.to_rgb666_bytes();
        assert_eq!(check_frame(&spi, |_, _| blue), w * h, "{rotation:?}");

        // Same pixel-aligned 4095-byte transfers as draw_image
        let transfers = &spi.0.borrow().pixel_transfers;
        assert_eq!(transfers.len(), (w * h * 3).div_ceil(4095), "{rotation:?}");
        assert!(transfers[..transfers.len() - 1].iter().all(|&n| n == 4095));
    }
}

#[test]
fn fill_rectangle_overwrites() {
    let (mut display, spi) = landscape();
    display.fill_rectangle(0, 0, 100, 100, Color::RED).unwrap();
    display.fill_rectangle(50, 50, 100, 100, Color::BLUE).unwrap();

    let (red, blue) = (Color::RED.to_rgb666_bytes(), Color::BLUE.to_rgb666_bytes());
    check_frame(&spi, |x, y| {
        if inside(x, y, 50, 50, 100, 100) {
            blue
        } else if inside(x, y, 0, 0, 100, 100) {
            red
        } else {
            BLACK
        }
    });
}

// ---- draw_text ----

/// How a piece of text was drawn, for working out the expected pixels.
struct TextAt<'a> {
    text: &'a str,
    x: usize,
    y: usize,
    size: usize,
    fg: Color,
    bg: Color,
}

impl TextAt<'_> {
    /// Expected color of pixel (x, y), built from the font's column bytes
    /// (bit 0 = top row). `None` outside the text's character cells.
    fn pixel(&self, x: usize, y: usize) -> Option<[u8; 3]> {
        let (cell_w, cell_h) = (6 * self.size, 8 * self.size);
        for (line_index, line) in self.text.split('\n').enumerate() {
            let line_y = self.y + line_index * cell_h;
            for (char_index, c) in line.chars().enumerate() {
                let cell_x = self.x + char_index * cell_w;
                if inside(x, y, cell_x, line_y, cell_w, cell_h) {
                    let (col, row) = ((x - cell_x) / self.size, (y - line_y) / self.size);
                    // Column 5 is the 1-pixel gap after the glyph
                    let on = col < 5 && (font::glyph(c)[col] >> row) & 1 == 1;
                    let color = if on { self.fg } else { self.bg };
                    return Some(color.to_rgb666_bytes());
                }
            }
        }
        None
    }

    /// Checks the whole panel shows just this text. Returns pixels sent.
    fn check(&self, spi: &MockSpi) -> usize {
        check_frame(spi, |x, y| self.pixel(x, y).unwrap_or(BLACK))
    }
}

#[test]
fn draw_text_single_character() {
    let (mut display, spi) = landscape();
    display.draw_text(5, 7, "A", Color::WHITE, Color::BLUE, 1).unwrap();
    let text = TextAt { text: "A", x: 5, y: 7, size: 1, fg: Color::WHITE, bg: Color::BLUE };
    assert_eq!(text.check(&spi), 6 * 8, "one 6x8 cell");

    // Spot check 'A' (0x7C, 0x12, 0x11, 0x12, 0x7C): its top row is only the middle column
    let p = spi.0.borrow();
    let at = |x: usize, y: usize| p.pixels[y * p.width + x];
    let (white, blue) = (Color::WHITE.to_rgb666_bytes(), Color::BLUE.to_rgb666_bytes());
    assert_eq!([at(5, 7), at(6, 7), at(7, 7), at(8, 7), at(9, 7)], [blue, blue, white, blue, blue]);
    assert_eq!(at(10, 7), blue, "gap column");
}

#[test]
fn draw_text_scaled() {
    for size in [2u8, 3, 4] {
        let (mut display, spi) = landscape();
        display.draw_text(20, 30, "Rust!", Color::YELLOW, Color::BLACK, size).unwrap();
        let s = usize::from(size);
        let text = TextAt { text: "Rust!", x: 20, y: 30, size: s, fg: Color::YELLOW, bg: Color::BLACK };
        assert_eq!(text.check(&spi), 5 * (6 * s) * (8 * s), "size {size}");
    }
}

#[test]
fn draw_text_newlines_and_descenders() {
    let s = "gyp\nAB\n\nq";
    let (mut display, spi) = landscape();
    display.draw_text(3, 4, s, Color::GREEN, Color::GREY, 2).unwrap();
    TextAt { text: s, x: 3, y: 4, size: 2, fg: Color::GREEN, bg: Color::GREY }.check(&spi);
}

#[test]
fn draw_text_size_zero_is_size_one() {
    let (mut display, spi) = landscape();
    display.draw_text(0, 0, "x", Color::WHITE, Color::RED, 0).unwrap();
    TextAt { text: "x", x: 0, y: 0, size: 1, fg: Color::WHITE, bg: Color::RED }.check(&spi);
}

#[test]
fn draw_text_unknown_characters_draw_question_mark() {
    let (mut display, spi) = landscape();
    display.draw_text(0, 0, "\u{e9}", Color::WHITE, Color::RED, 1).unwrap();
    TextAt { text: "?", x: 0, y: 0, size: 1, fg: Color::WHITE, bg: Color::RED }.check(&spi);
}

#[test]
fn draw_text_clipped_at_edges() {
    // Runs off the right edge: "WWW" at x = 470 leaves 10 visible columns
    let (mut display, spi) = landscape();
    display.draw_text(470, 0, "WWW", Color::WHITE, Color::BLUE, 1).unwrap();
    let text = TextAt { text: "WWW", x: 470, y: 0, size: 1, fg: Color::WHITE, bg: Color::BLUE };
    assert_eq!(text.check(&spi), 10 * 8);

    // Runs off the bottom: size 2 at y = 310 leaves 10 visible rows
    let (mut display, spi) = landscape();
    display.draw_text(0, 310, "Hi", Color::WHITE, Color::BLUE, 2).unwrap();
    let text = TextAt { text: "Hi", x: 0, y: 310, size: 2, fg: Color::WHITE, bg: Color::BLUE };
    assert_eq!(text.check(&spi), 2 * 12 * 10);

    // Entirely off screen
    let (mut display, spi) = landscape();
    display.draw_text(480, 0, "off", Color::WHITE, Color::BLUE, 1).unwrap();
    display.draw_text(0, 320, "off", Color::WHITE, Color::BLUE, 1).unwrap();
    assert!(spi.0.borrow().pixel_transfers.is_empty());
}

#[test]
fn text_width() {
    type Display = TftDisplay<MockSpi>;
    assert_eq!(Display::text_width("", 1), 0);
    assert_eq!(Display::text_width("abc", 1), 18);
    assert_eq!(Display::text_width("abc", 2), 36);
    assert_eq!(Display::text_width("abc", 0), 18, "size 0 counts as 1");
    assert_eq!(Display::text_width("ab\nlonger\nc", 1), 36, "longest line");
}

/// Windows line endings ("\r\n") draw the same as "\n".
#[test]
fn draw_text_ignores_carriage_returns() {
    let (mut display, spi) = landscape();
    display.draw_text(2, 3, "one\r\ntwo\r\n\r\nfour", Color::WHITE, Color::RED, 2).unwrap();
    TextAt { text: "one\ntwo\n\nfour", x: 2, y: 3, size: 2, fg: Color::WHITE, bg: Color::RED }.check(&spi);
}

/// text_width agrees with what draw_text actually covers (character cells
/// drawn in the background color), including odd line endings.
#[test]
fn text_width_matches_drawn_width() {
    for text in ["abc", "ab\nlonger\nc", "ab\r\ncd", "trailing\n", "\n\nx", "tab\there", "\u{e9}t\u{e9}"] {
        let (mut display, spi) = landscape();
        display.draw_text(0, 0, text, Color::WHITE, Color::BLUE, 1).unwrap();
        let p = spi.0.borrow();
        let drawn_width = (0..p.width)
            .filter(|&x| (0..p.height).any(|y| p.pixels[y * p.width + x] != BLACK))
            .map(|x| x + 1)
            .max()
            .unwrap_or(0);
        assert_eq!(
            usize::from(TftDisplay::<MockSpi>::text_width(text, 1)),
            drawn_width,
            "{text:?}"
        );
    }
}

// ---- rotations ----

const ROTATIONS: [Rotate; 4] = [Rotate::Degrees0, Rotate::Degrees90, Rotate::Degrees180, Rotate::Degrees270];

/// A display on a mock panel sized for `rotation`'s logical size.
fn rotated(rotation: Rotate) -> (TftDisplay<MockSpi>, MockSpi) {
    let (w, h) = match rotation {
        Rotate::Degrees0 | Rotate::Degrees180 => (320, 480),
        Rotate::Degrees90 | Rotate::Degrees270 => (480, 320),
    };
    let spi = MockSpi::new(w, h);
    let mut display = TftDisplay::new(spi.clone());
    display.set_rotation(rotation).unwrap();
    (display, spi)
}

/// Where a pixel of `color` ended up in GRAM. Panics unless exactly one did.
fn find_in_gram(spi: &MockSpi, color: Color) -> (i32, i32) {
    let target = color.to_rgb666_bytes();
    let p = spi.0.borrow();
    let found: Vec<usize> = (0..p.gram.len()).filter(|&i| p.gram[i] == target).collect();
    assert_eq!(found.len(), 1, "expected one {color} pixel in GRAM");
    ((found[0] % GRAM_W) as i32, (found[0] / GRAM_W) as i32)
}

/// How a rotation lays the logical screen onto GRAM: where logical (0, 0)
/// lands, and the GRAM step for +1 in logical x and in logical y.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Orientation {
    origin: (i32, i32),
    x_step: (i32, i32),
    y_step: (i32, i32),
}

fn orientation(rotation: Rotate) -> Orientation {
    let (mut display, spi) = rotated(rotation);
    display.draw_pixel(0, 0, Color::RED).unwrap();
    display.draw_pixel(1, 0, Color::GREEN).unwrap();
    display.draw_pixel(0, 1, Color::BLUE).unwrap();
    let origin = find_in_gram(&spi, Color::RED);
    let (gx, gy) = find_in_gram(&spi, Color::GREEN);
    let (bx, by) = find_in_gram(&spi, Color::BLUE);
    Orientation {
        origin,
        x_step: (gx - origin.0, gy - origin.1),
        y_step: (bx - origin.0, by - origin.1),
    }
}

/// Quarter turn of a GRAM step vector: (dx, dy) -> (-dy, dx).
fn quarter_turn((dx, dy): (i32, i32)) -> (i32, i32) {
    (-dy, dx)
}

/// The MADCTL values were checked on the actual panel with tft-spi-demo's
/// rotation test (green block in the bottom-left corner in every rotation).
/// This pins them so a refactor can't silently change the orientation.
#[test]
fn rotation_madctl_values_match_hardware() {
    for (rotation, expected) in [
        (Rotate::Degrees0, 0x88),
        (Rotate::Degrees90, 0x28),
        (Rotate::Degrees180, 0x48),
        (Rotate::Degrees270, 0xE8),
    ] {
        let (_display, spi) = rotated(rotation);
        let p = spi.0.borrow();
        assert_eq!(p.commands.last(), Some(&(MADCTL, vec![expected])), "{rotation:?}");
        assert!(p.pixel_transfers.is_empty(), "{rotation:?}: set_rotation sent pixels");
    }
}

#[test]
fn rotation_dimensions_and_degrees() {
    for (rotation, size, degrees) in [
        (Rotate::Degrees0, (320, 480), 0),
        (Rotate::Degrees90, (480, 320), 90),
        (Rotate::Degrees180, (320, 480), 180),
        (Rotate::Degrees270, (480, 320), 270),
    ] {
        let (display, _spi) = rotated(rotation);
        assert_eq!((display.width(), display.height()), size, "{rotation:?}");
        assert_eq!(display.rotation(), rotation);
        assert_eq!(rotation.degrees(), degrees);
        assert_eq!(Rotate::from_degrees(degrees), Some(rotation));
    }
    assert_eq!(Rotate::from_degrees(45), None);
}

/// In every rotation the logical screen covers all of GRAM, each pixel once.
#[test]
fn rotation_fill_covers_whole_panel_once() {
    for rotation in ROTATIONS {
        let (mut display, spi) = rotated(rotation);
        display.fill_screen(Color::WHITE).unwrap();
        let p = spi.0.borrow();
        assert!(p.gram_writes.iter().all(|&n| n == 1), "{rotation:?}");
    }
}

/// The four rotations are quarter turns of each other, all in the same
/// direction, with no mirror images. (Which way is clockwise depends on how
/// the glass is mounted, so only consistency is checked.)
#[test]
fn rotations_are_consistent_quarter_turns() {
    let orientations = ROTATIONS.map(orientation);
    let corners = [(0, 0), (GRAM_W as i32 - 1, 0), (0, GRAM_H as i32 - 1), (GRAM_W as i32 - 1, GRAM_H as i32 - 1)];
    // Cross product of the x and y steps: +1 or -1 depending on handedness
    let handedness = |o: &Orientation| o.x_step.0 * o.y_step.1 - o.x_step.1 * o.y_step.0;

    for (rotation, o) in ROTATIONS.iter().zip(&orientations) {
        assert!(corners.contains(&o.origin), "{rotation:?}: logical (0, 0) at {:?}, not a corner", o.origin);
        assert_eq!(o.x_step.0.abs() + o.x_step.1.abs(), 1, "{rotation:?}: x step {:?}", o.x_step);
        assert_eq!(o.y_step.0.abs() + o.y_step.1.abs(), 1, "{rotation:?}: y step {:?}", o.y_step);
        assert_eq!(handedness(o), handedness(&orientations[0]), "{rotation:?} is mirrored relative to Degrees0");
    }

    // Each step to the next rotation turns the axes the same way
    let turn = |v: (i32, i32), clockwise: bool| {
        if clockwise {
            quarter_turn(v)
        } else {
            quarter_turn(quarter_turn(quarter_turn(v)))
        }
    };
    let clockwise = orientations[1].x_step == quarter_turn(orientations[0].x_step);
    for k in 0..4 {
        let (from, to) = (&orientations[k], &orientations[(k + 1) % 4]);
        assert_eq!(to.x_step, turn(from.x_step, clockwise), "{:?} -> {:?}", ROTATIONS[k], ROTATIONS[(k + 1) % 4]);
        assert_eq!(to.y_step, turn(from.y_step, clockwise), "{:?} -> {:?}", ROTATIONS[k], ROTATIONS[(k + 1) % 4]);
    }

    // The origins visit all four corners, and half turns swap opposite corners
    let mut origins: Vec<_> = orientations.iter().map(|o| o.origin).collect();
    let opposite = |(c, r): (i32, i32)| (GRAM_W as i32 - 1 - c, GRAM_H as i32 - 1 - r);
    assert_eq!(orientations[2].origin, opposite(orientations[0].origin));
    assert_eq!(orientations[3].origin, opposite(orientations[1].origin));
    origins.sort();
    origins.dedup();
    assert_eq!(origins.len(), 4, "logical (0, 0) should be a different corner in each rotation");
}

/// Something drawn in Degrees180 is the Degrees0 drawing turned upside down
/// (and the same for Degrees270 vs Degrees90).
#[test]
fn half_turn_rotations_draw_upside_down() {
    let draw = |rotation: Rotate| {
        let (mut display, spi) = rotated(rotation);
        display.draw_text(4, 6, "Fg", Color::WHITE, Color::BLUE, 3).unwrap();
        display.fill_round_rectangle(30, 70, 50, 25, 8, Color::RED).unwrap();
        display.draw_line(0, 0, 100, 40, Color::GREEN).unwrap();
        let gram = spi.0.borrow().gram.clone();
        gram
    };
    let upside_down = |gram: &[[u8; 3]]| -> Vec<[u8; 3]> {
        (0..GRAM_W * GRAM_H)
            .map(|i| gram[GRAM_W * GRAM_H - 1 - i])
            .collect()
    };

    for (a, b) in [(Rotate::Degrees0, Rotate::Degrees180), (Rotate::Degrees90, Rotate::Degrees270)] {
        let (gram_a, gram_b) = (draw(a), draw(b));
        assert!(gram_a.iter().any(|&px| px != BLACK));
        assert!(upside_down(&gram_a) == gram_b, "{b:?} is not {a:?} turned upside down");
    }
}

#[test]
fn rotation_address_bounds() {
    for rotation in ROTATIONS {
        let (mut display, spi) = rotated(rotation);
        let (w, h) = (display.width(), display.height());
        // Opposite corners are drawable
        display.set_addr_window(0, 0, w - 1, h - 1).unwrap();
        // One past the edge is rejected before anything is sent
        let sent_before = spi.0.borrow().commands.len();
        assert!(matches!(display.set_addr_window(0, 0, w, h - 1), Err(Error::Size { .. })), "{rotation:?}");
        assert!(matches!(display.set_addr_window(0, 0, w - 1, h), Err(Error::Size { .. })), "{rotation:?}");
        assert_eq!(spi.0.borrow().commands.len(), sent_before, "{rotation:?}");
    }
}

// ---- shapes through the display ----

/// Draws a shape on a landscape display and checks the panel shows exactly
/// the on-screen part of what `shapes` produces for it.
fn check_clipped_shape(
    name: &str,
    draw: impl FnOnce(&mut TftDisplay<MockSpi>, Color),
    raster: impl FnOnce(&mut dyn FnMut(shapes::Rect) -> std::result::Result<(), ()>),
) {
    let mut expected = std::collections::HashSet::new();
    raster(&mut |(x, y, w, h)| {
        for px in x.max(0)..(x + w).min(480) {
            for py in y.max(0)..(y + h).min(320) {
                expected.insert((px as usize, py as usize));
            }
        }
        Ok(())
    });

    let (mut display, spi) = landscape();
    draw(&mut display, Color::CYAN);
    let cyan = Color::CYAN.to_rgb666_bytes();
    check_frame(&spi, |x, y| if expected.contains(&(x, y)) { cyan } else { BLACK });
    assert!(!expected.is_empty(), "{name}: test shape should be partly visible");
}

#[test]
fn lines_clipped_at_edges() {
    for (x0, y0, x1, y1) in [
        (-100, -40, 600, 400),  // through the screen, both ends off it
        (240, 160, 900, 170),   // off the right
        (240, 160, 250, -900),  // off the top, steep
        (-5, 319, 484, 319),    // along the bottom row
        (479, -10, 479, 330),   // along the right column
        (-30, 40, 40, -30),     // clipping the top-left corner
    ] {
        check_clipped_shape(
            &format!("line {:?}", (x0, y0, x1, y1)),
            |d, c| d.draw_line(x0, y0, x1, y1, c).unwrap(),
            |emit| shapes::line(x0, y0, x1, y1, emit).unwrap(),
        );
    }
}

#[test]
fn circles_clipped_at_edges() {
    for (cx, cy, r) in [(0, 0, 50), (479, 319, 30), (240, -20, 40), (-10, 160, 60), (240, 160, 280)] {
        check_clipped_shape(
            &format!("circle {:?}", (cx, cy, r)),
            |d, c| d.draw_circle(cx, cy, r, c).unwrap(),
            |emit| shapes::circle(cx, cy, r, emit).unwrap(),
        );
        check_clipped_shape(
            &format!("filled circle {:?}", (cx, cy, r)),
            |d, c| d.fill_circle(cx, cy, r, c).unwrap(),
            |emit| shapes::filled_circle(cx, cy, r, emit).unwrap(),
        );
    }
}

#[test]
fn rectangles_clipped_at_edges() {
    for (x, y, w, h, r) in [(-20, -20, 100, 60, 10), (440, 290, 100, 100, 20), (-10, 100, 500, 50, 25), (5, 5, 470, 310, 0)] {
        check_clipped_shape(
            &format!("rectangle {:?}", (x, y, w, h)),
            |d, c| d.draw_rectangle(x, y, w, h, c).unwrap(),
            |emit| shapes::rectangle(x, y, w, h, emit).unwrap(),
        );
        check_clipped_shape(
            &format!("round rectangle {:?}", (x, y, w, h, r)),
            |d, c| d.fill_round_rectangle(x, y, w, h, r, c).unwrap(),
            |emit| shapes::filled_round_rectangle(x, y, w, h, r, emit).unwrap(),
        );
    }
}

#[test]
fn draw_pixel_on_and_off_screen() {
    let (mut display, spi) = landscape();
    for (x, y) in [(0, 0), (479, 319), (100, 200), (-1, 0), (0, -1), (480, 0), (0, 320)] {
        display.draw_pixel(x, y, Color::WHITE).unwrap();
    }
    let white = Color::WHITE.to_rgb666_bytes();
    let on = [(0, 0), (479, 319), (100, 200)];
    let sent = check_frame(&spi, |x, y| if on.contains(&(x, y)) { white } else { BLACK });
    assert_eq!(sent, 3);
}

/// Shapes far outside the screen, near the i32 limits, draw nothing and
/// don't overflow.
#[test]
fn shapes_at_extreme_coordinates() {
    let (mut display, spi) = landscape();
    let (max, min) = (i32::MAX, i32::MIN);
    display.draw_pixel(max, min, Color::WHITE).unwrap();
    display.draw_line(max - 10, max - 10, max, max, Color::WHITE).unwrap();
    display.draw_line(min, min, min + 10, min + 3, Color::WHITE).unwrap();
    display.draw_circle(max, max, u16::MAX, Color::WHITE).unwrap();
    display.draw_circle(min, min, u16::MAX, Color::WHITE).unwrap();
    display.fill_circle(max, min, u16::MAX, Color::WHITE).unwrap();
    display.draw_rectangle(max, max, u16::MAX, u16::MAX, Color::WHITE).unwrap();
    display.fill_round_rectangle(max - 5, max - 5, u16::MAX, u16::MAX, 100, Color::WHITE).unwrap();
    display.draw_image(max - 1, max - 1, 2, 2, &[0; 12]).unwrap();
    assert!(spi.0.borrow().pixel_transfers.is_empty());
}

// ---- initialization ----

/// The init sequence from tft-spi-demo, which works on the actual panel.
/// Pinned so changes to it are deliberate.
#[test]
fn initialize_sends_known_working_sequence() {
    let spi = MockSpi::new(320, 480);
    let mut display = TftDisplay::new(spi.clone());
    display.initialize().unwrap();

    let gamma_negative = vec![
        0x0F, 0x32, 0x2E, 0x0B, 0x0D, 0x05, 0x47, 0x75, 0x37, 0x06, 0x10, 0x03, 0x24, 0x20, 0x00,
    ];
    let expected: Vec<(u8, Vec<u8>)> = vec![
        (0x01, vec![]),                 // soft reset
        (0xB1, vec![0x10, 0x11]),       // frame rate, normal mode: 30 Hz, 17 clocks/line
        (0xB2, vec![0x00, 0x11]),       // frame rate, idle mode
        (0xB3, vec![0x00, 0x11]),       // frame rate, partial mode
        (0xC2, vec![0x44]),             // power control 3
        (0xC5, vec![0x00, 0x00, 0x00, 0x00]), // VCOM control
        (0x3A, vec![0x66]),             // pixel format: 18 bits/pixel
        (0xB0, vec![0x00]),             // interface mode control
        (
            0xE0, // positive gamma
            vec![0x0F, 0x1F, 0x1C, 0x0C, 0x0F, 0x08, 0x48, 0x98, 0x37, 0x0A, 0x13, 0x04, 0x11, 0x0D, 0x00],
        ),
        (0xE1, gamma_negative.clone()), // negative gamma
        (0xE2, gamma_negative),         // digital gamma
        (0x11, vec![]),                 // sleep out
        (0x29, vec![]),                 // display on
        (0x36, vec![0x88]),             // MADCTL for the default rotation (Degrees0)
    ];

    let p = spi.0.borrow();
    assert_eq!(p.commands, expected);
    assert_eq!(p.resets, vec![0], "one hardware reset, before any command");
    assert!(p.pixel_transfers.is_empty(), "initialize sends no pixels");
    assert_eq!((display.width(), display.height(), display.rotation()), (320, 480, Rotate::Degrees0));
}

/// The 3-byte pixels the driver sends only work in 18-bit mode.
#[test]
fn initialize_selects_18_bit_pixels() {
    let spi = MockSpi::new(320, 480);
    let mut display = TftDisplay::new(spi.clone());
    display.initialize().unwrap();
    let p = spi.0.borrow();
    let format = p.commands.iter().find(|(cmd, _)| *cmd == Command::InterfacePixelFormat as u8);
    assert_eq!(format, Some(&(0x3A, vec![0x66])), "0x66 = 18 bits/pixel (RGB666)");
}

/// ILI9486 timing: wait 5 ms after a soft reset and 120 ms after sleep out
/// before sending more commands.
#[test]
fn initialize_waits_after_reset_and_sleep_out() {
    let spi = MockSpi::new(320, 480);
    let mut display = TftDisplay::new(spi.clone());
    display.initialize().unwrap();
    let p = spi.0.borrow();
    let delay = |cmd: Command| {
        p.delays
            .iter()
            .find(|(c, _)| *c == cmd as u8)
            .map(|&(_, d)| d)
            .unwrap_or_else(|| panic!("no delay after {cmd:?}"))
    };
    assert!(delay(Command::SoftReset) >= Duration::from_millis(5));
    assert!(delay(Command::SleepOut) >= Duration::from_millis(120));
}

/// initialize keeps a rotation chosen beforehand.
#[test]
fn initialize_applies_current_rotation() {
    let spi = MockSpi::new(480, 320);
    let mut display = TftDisplay::new(spi.clone());
    display.set_rotation(Rotate::Degrees270).unwrap();
    display.initialize().unwrap();
    let p = spi.0.borrow();
    assert_eq!(p.commands.last(), Some(&(MADCTL, vec![0xE8])));
    assert_eq!((display.width(), display.height()), (480, 320));
}

/// Changing rotation only affects what's drawn afterwards.
#[test]
fn rotation_change_keeps_existing_pixels() {
    let spi = MockSpi::new(480, 480); // big enough for both logical sizes
    let mut display = TftDisplay::new(spi.clone());
    display.set_rotation(Rotate::Degrees0).unwrap();
    display.fill_rectangle(0, 0, 10, 10, Color::RED).unwrap();
    let before = spi.0.borrow().gram.clone();

    display.set_rotation(Rotate::Degrees90).unwrap();
    assert!(spi.0.borrow().gram == before, "set_rotation changed GRAM");
    display.fill_rectangle(0, 0, 10, 10, Color::BLUE).unwrap();

    let p = spi.0.borrow();
    let count = |c: Color| p.gram.iter().filter(|&&px| px == c.to_rgb666_bytes()).count();
    assert_eq!(count(Color::RED), 100, "first square still there");
    assert_eq!(count(Color::BLUE), 100, "second square in a different corner");
}
