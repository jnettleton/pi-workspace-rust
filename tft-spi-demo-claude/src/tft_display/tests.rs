//! Tests for `TftDisplay` against a mock ILI9486: the mock decodes the
//! column/row address and memory write commands the display sends and paints
//! the pixels into a framebuffer, so tests can check what would be on screen.
//! Runs on any platform with `cargo test`.

// `% 3 == 0` and `chunks_exact` rather than `is_multiple_of` / `as_chunks`
// (Rust 1.87 / 1.88) so the tests also build on older Pi toolchains.
#![allow(clippy::manual_is_multiple_of, clippy::chunks_exact_to_as_chunks)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use super::{
    color::Color,
    enums::{Command, Rotate},
    error::Error,
    font,
    tft_display::{Result, TftDisplay},
};
use crate::tft_spi::TftSpi;

/// What the mock panel has received.
#[derive(Default)]
struct Panel {
    width: usize,
    height: usize,
    /// RGB666 bytes per pixel, as sent
    pixels: Vec<[u8; 3]>,
    cmd: u8,
    params: Vec<u8>,
    /// Address window: x0, x1, y0, y1 (inclusive)
    window: (usize, usize, usize, usize),
    cursor: (usize, usize),
    pixels_written: usize,
    /// Pixels sent after the window was already full
    overflow: usize,
    /// Byte count of each memory write data transfer
    pixel_transfers: Vec<usize>,
}

/// Shared handle so the test can inspect the panel after the display has
/// taken ownership of its `TftSpi`.
#[derive(Clone)]
struct MockSpi(Rc<RefCell<Panel>>);

impl MockSpi {
    fn new(width: usize, height: usize) -> Self {
        MockSpi(Rc::new(RefCell::new(Panel {
            width,
            height,
            pixels: vec![[0; 3]; width * height],
            ..Default::default()
        })))
    }
}

const CASET: u8 = Command::ColumnAddressSet as u8;
const RASET: u8 = Command::RowAddressSet as u8;
const RAMWR: u8 = Command::MemoryWrite as u8;

impl TftSpi for MockSpi {
    fn reset_pin(&mut self) {}

    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> Result<usize> {
        let n = self.write_command(cmd)?;
        Ok(n + self.write_data(params)?)
    }

    fn write_command(&mut self, cmd: Command) -> Result<usize> {
        let mut p = self.0.borrow_mut();
        p.cmd = cmd as u8;
        p.params.clear();
        if p.cmd == RAMWR {
            p.cursor = (p.window.0, p.window.2);
        }
        Ok(1)
    }

    fn write_command_delay(&mut self, cmd: Command, _delay: Duration) -> Result<usize> {
        self.write_command(cmd)
    }

    fn write_data(&mut self, data: &[u8]) -> Result<usize> {
        let mut p = self.0.borrow_mut();
        let be16 = |b: &[u8]| usize::from(u16::from_be_bytes([b[0], b[1]]));
        match p.cmd {
            CASET | RASET => {
                p.params.extend_from_slice(data);
                if p.params.len() == 4 {
                    let (start, end) = (be16(&p.params[0..2]), be16(&p.params[2..4]));
                    if p.cmd == CASET {
                        (p.window.0, p.window.1) = (start, end);
                    } else {
                        (p.window.2, p.window.3) = (start, end);
                    }
                }
            }
            RAMWR => {
                assert!(data.len() % 3 == 0, "transfer of {} bytes splits a pixel", data.len());
                assert!(data.len() <= 4096, "transfer of {} bytes exceeds spidev bufsiz", data.len());
                p.pixel_transfers.push(data.len());
                for px in data.chunks_exact(3) {
                    assert!(px.iter().all(|b| b & 0x03 == 0), "low 2 bits set in {px:?}");
                    let (x, y) = p.cursor;
                    if y > p.window.3 {
                        p.overflow += 1;
                        continue;
                    }
                    let i = y * p.width + x;
                    p.pixels[i] = [px[0], px[1], px[2]];
                    p.pixels_written += 1;
                    p.cursor = if x == p.window.1 { (p.window.0, y + 1) } else { (x + 1, y) };
                }
            }
            _ => {}
        }
        Ok(data.len())
    }
}

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
