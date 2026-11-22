use crate::tft_display::{
    color::Color,
    enums::{madctl, Command, Rotate},
    error::Error,
    font, shapes,
};
use crate::tft_spi::TftSpi;

use std::{result, thread, time::Duration};

pub type Result<T> = result::Result<T, Error>;

/// Bytes per pixel in 18-bit mode
const BYTES_PER_PIXEL: usize = 3;

/// spidev's default `bufsiz` is 4096 bytes per transfer. Rounded down to a
/// whole number of pixels so a pixel is never split across two transfers.
const MAX_BUFFER_SIZE: usize = (4096 / BYTES_PER_PIXEL) * BYTES_PER_PIXEL;

/// Native (portrait) panel size of the ILI9486
const NATIVE_WIDTH: u16 = 320;
const NATIVE_HEIGHT: u16 = 480;

/// Driver for the ILI9486 on the Waveshare 3.5" RPi LCD (A), 18 bits per pixel.
pub struct TftDisplay<S: TftSpi> {
    tft_spi: S,
    width: u16,
    height: u16,
    rotation: Rotate,
}

impl<S: TftSpi> TftDisplay<S> {
    pub fn new(tft_spi: S) -> Self {
        Self {
            tft_spi,
            width: NATIVE_WIDTH,
            height: NATIVE_HEIGHT,
            rotation: Rotate::Degrees0,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn rotation(&self) -> Rotate {
        self.rotation
    }

    /// Init sequence from tft-spi-demo (the one that works on the board).
    pub fn initialize(&mut self) -> Result<()> {
        self.tft_spi.reset_pin();

        let spi = &mut self.tft_spi;
        spi.write_command_delay(Command::SoftReset, Duration::from_millis(150))?;

        // Frame rate: 30 Hz, division ratio fosc, 17 clocks per line
        let frs = 0b0001; // 30 Hz
        let div = 0b0000; // fosc
        let rtn = 0x11; // 17 clocks per line
        spi.write_reg(Command::FrameRateControlNormal, &[(frs << 4) | div, rtn])?;
        thread::sleep(Duration::from_millis(10));
        spi.write_reg(Command::FrameRateControlIdle, &[div, rtn])?;
        spi.write_reg(Command::FrameRateControlPartial, &[div, rtn])?;

        // Step-up circuit frequencies in normal mode: 4 H / 4 H
        let dc0 = 0b0100;
        let dc1 = 0b0100;
        spi.write_reg(Command::PowerControl3, &[(dc1 << 4) | dc0])?;

        spi.write_reg(Command::VcomControl1, &[0x00, 0x00, 0x00, 0x00])?;

        // 18 bits / pixel for DPI and DBI (the only color depth ILI9486 supports over SPI)
        let dpi = 0b0110;
        let dbi = 0b0110;
        spi.write_reg(Command::InterfacePixelFormat, &[(dpi << 4) | dbi])?;
        spi.write_reg(Command::InterfaceModeControl, &[0x00])?;

        spi.write_reg(
            Command::PositiveGammaControl,
            &[
                0x0F, 0x1F, 0x1C, 0x0C, 0x0F, 0x08, 0x48, 0x98, 0x37, 0x0A, 0x13, 0x04, 0x11,
                0x0D, 0x00,
            ],
        )?;
        spi.write_reg(
            Command::NegativeGammaControl,
            &[
                0x0F, 0x32, 0x2E, 0x0B, 0x0D, 0x05, 0x47, 0x75, 0x37, 0x06, 0x10, 0x03, 0x24,
                0x20, 0x00,
            ],
        )?;
        spi.write_reg(
            Command::DigitalGammaControl1,
            &[
                0x0F, 0x32, 0x2E, 0x0B, 0x0D, 0x05, 0x47, 0x75, 0x37, 0x06, 0x10, 0x03, 0x24,
                0x20, 0x00,
            ],
        )?;

        spi.write_command_delay(Command::SleepOut, Duration::from_millis(120))?;
        spi.write_command_delay(Command::DisplayOn, Duration::from_millis(100))?;

        self.set_rotation(self.rotation)?;

        Ok(())
    }

    /// Sets the orientation (MADCTL values from tft-spi-demo).
    pub fn set_rotation(&mut self, rotation: Rotate) -> Result<()> {
        let (value, width, height) = match rotation {
            Rotate::Degrees0 => (madctl::MY, NATIVE_WIDTH, NATIVE_HEIGHT),
            Rotate::Degrees90 => (madctl::MV, NATIVE_HEIGHT, NATIVE_WIDTH),
            Rotate::Degrees180 => (madctl::MX, NATIVE_WIDTH, NATIVE_HEIGHT),
            Rotate::Degrees270 => (
                madctl::MX | madctl::MY | madctl::MV,
                NATIVE_HEIGHT,
                NATIVE_WIDTH,
            ),
        };

        self.tft_spi
            .write_reg(Command::MemoryAccessControl, &[value | madctl::BGR])?;

        self.rotation = rotation;
        self.width = width;
        self.height = height;
        Ok(())
    }

    pub fn fill_screen(&mut self, color: Color) -> Result<()> {
        self.fill_rectangle(0, 0, self.width, self.height, color)
    }

    /// Fills a rectangle, clipped to the screen.
    pub fn fill_rectangle(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) -> Result<()> {
        if w == 0 || h == 0 || x >= self.width || y >= self.height {
            return Ok(());
        }

        let w = w.min(self.width - x);
        let h = h.min(self.height - y);

        self.set_addr_window(x, y, x + w - 1, y + h - 1)?;
        self.tft_spi.write_command(Command::MemoryWrite)?;

        // One buffer full of the color, written as many times as needed.
        let pixel = color.to_rgb666_bytes();
        let buffer: Vec<u8> = pixel.iter().copied().cycle().take(MAX_BUFFER_SIZE).collect();

        let mut remaining = w as usize * h as usize * BYTES_PER_PIXEL;
        while remaining > 0 {
            let n = remaining.min(MAX_BUFFER_SIZE);
            self.tft_spi.write_data(&buffer[..n])?;
            remaining -= n;
        }

        Ok(())
    }

    /// Sets one pixel. Off-screen coordinates are ignored.
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) -> Result<()> {
        self.fill_rect_clipped((i64::from(x), i64::from(y), 1, 1), color)
    }

    /// Draws a 1-pixel line from (x0, y0) to (x1, y1), both ends included.
    /// Coordinates may be off-screen; the line is clipped.
    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) -> Result<()> {
        shapes::line(x0, y0, x1, y1, |rect| self.fill_rect_clipped(rect, color))
    }

    /// Draws a 1-pixel rectangle outline with its top-left corner at (x, y).
    /// Clipped to the screen. Use `fill_rectangle` for a solid one.
    pub fn draw_rectangle(&mut self, x: i32, y: i32, w: u16, h: u16, color: Color) -> Result<()> {
        shapes::rectangle(x, y, w, h, |rect| self.fill_rect_clipped(rect, color))
    }

    /// Fills a rectangle with rounded corners, top-left corner at (x, y).
    /// `radius` is reduced to fit if it's more than half the shorter side
    /// (so a large radius gives a pill shape). Clipped to the screen.
    pub fn fill_round_rectangle(
        &mut self,
        x: i32,
        y: i32,
        w: u16,
        h: u16,
        radius: u16,
        color: Color,
    ) -> Result<()> {
        shapes::filled_round_rectangle(x, y, w, h, radius, |rect| self.fill_rect_clipped(rect, color))
    }

    /// Draws a 1-pixel circle outline centered on (cx, cy). Clipped to the screen.
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: u16, color: Color) -> Result<()> {
        shapes::circle(cx, cy, radius, |rect| self.fill_rect_clipped(rect, color))
    }

    /// Draws a filled circle centered on (cx, cy). Clipped to the screen.
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: u16, color: Color) -> Result<()> {
        shapes::filled_circle(cx, cy, radius, |rect| self.fill_rect_clipped(rect, color))
    }

    /// Draws an image with its top-left corner at (x, y). `rgb` holds
    /// `width * height` pixels, row by row, 3 bytes each (R, G, B, 8 bits per
    /// channel), which is what `image::RgbImage` stores. Clipped to the screen.
    pub fn draw_image(&mut self, x: i32, y: i32, width: u16, height: u16, rgb: &[u8]) -> Result<()> {
        let expected = usize::from(width) * usize::from(height) * 3;
        if rgb.len() != expected {
            return Err(Error::ImageSize { given: rgb.len(), expected });
        }
        let Some((x0, y0, x1, y1)) = self.clip((i64::from(x), i64::from(y), i64::from(width), i64::from(height))) else {
            return Ok(());
        };

        self.set_addr_window(x0, y0, x1 - 1, y1 - 1)?;
        self.tft_spi.write_command(Command::MemoryWrite)?;

        // Where the visible part starts in the source image
        let src_x = (i64::from(x0) - i64::from(x)) as usize;
        let src_y = (i64::from(y0) - i64::from(y)) as usize;
        let visible_bytes = usize::from(x1 - x0) * 3;
        let row_bytes = usize::from(width) * 3;

        let mut buffer = Vec::with_capacity(MAX_BUFFER_SIZE);
        // `chunks_exact` rather than `as_chunks` (Rust 1.88+) to build on older Pi toolchains
        #[allow(clippy::chunks_exact_to_as_chunks)]
        for row in 0..usize::from(y1 - y0) {
            let start = (src_y + row) * row_bytes + src_x * 3;
            for pixel in rgb[start..start + visible_bytes].chunks_exact(3) {
                buffer.extend_from_slice(&Color::rgb(pixel[0], pixel[1], pixel[2]).to_rgb666_bytes());
                // MAX_BUFFER_SIZE is a whole number of pixels, so this lands exactly
                if buffer.len() == MAX_BUFFER_SIZE {
                    self.tft_spi.write_data(&buffer)?;
                    buffer.clear();
                }
            }
        }
        if !buffer.is_empty() {
            self.tft_spi.write_data(&buffer)?;
        }

        Ok(())
    }

    /// `fill_rectangle` for signed, possibly off-screen rectangles.
    fn fill_rect_clipped(&mut self, rect: shapes::Rect, color: Color) -> Result<()> {
        match self.clip(rect) {
            Some((x0, y0, x1, y1)) => self.fill_rectangle(x0, y0, x1 - x0, y1 - y0, color),
            None => Ok(()),
        }
    }

    /// The on-screen part of a rectangle as `(x0, y0, x1, y1)`, end exclusive,
    /// or `None` if none of it is on screen.
    fn clip(&self, (x, y, w, h): shapes::Rect) -> Option<(u16, u16, u16, u16)> {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(i64::from(self.width));
        let y1 = (y + h).min(i64::from(self.height));
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some((x0 as u16, y0 as u16, x1 as u16, y1 as u16))
    }

    /// Draws `text` with its top-left corner at (`x`, `y`), scaled by `size`
    /// (1 = 6x8 pixels per character, 2 = 12x16, ...). `'\n'` starts a new line
    /// at `x`. Text running off the screen is clipped, not wrapped.
    pub fn draw_text(
        &mut self,
        x: u16,
        y: u16,
        text: &str,
        fg: Color,
        bg: Color,
        size: u8,
    ) -> Result<()> {
        let size = u16::from(size.max(1));
        let (mut cursor_x, mut cursor_y) = (x, y);

        for c in text.chars() {
            if c == '\n' {
                cursor_x = x;
                cursor_y = cursor_y.saturating_add(font::GLYPH_HEIGHT * size);
                continue;
            }
            self.draw_char(cursor_x, cursor_y, c, fg, bg, size as u8)?;
            cursor_x = cursor_x.saturating_add(font::CHAR_ADVANCE * size);
        }

        Ok(())
    }

    /// Width in pixels of the longest line of `text` at `size`.
    pub fn text_width(text: &str, size: u8) -> u16 {
        let longest = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        (longest as u16).saturating_mul(font::CHAR_ADVANCE * u16::from(size.max(1)))
    }

    /// Draws one character cell (glyph plus 1 column of spacing, in `bg`),
    /// clipped to the screen.
    pub fn draw_char(
        &mut self,
        x: u16,
        y: u16,
        c: char,
        fg: Color,
        bg: Color,
        size: u8,
    ) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }

        let size = u16::from(size.max(1));
        let w = (font::CHAR_ADVANCE * size).min(self.width - x);
        let h = (font::GLYPH_HEIGHT * size).min(self.height - y);

        let glyph = font::glyph(c);
        let fg = fg.to_rgb666_bytes();
        let bg = bg.to_rgb666_bytes();

        let mut buffer = Vec::with_capacity(w as usize * h as usize * BYTES_PER_PIXEL);
        for py in 0..h {
            let row = py / size;
            for px in 0..w {
                let col = (px / size) as usize;
                let on = glyph.get(col).is_some_and(|bits| (bits >> row) & 1 == 1);
                buffer.extend_from_slice(if on { &fg } else { &bg });
            }
        }

        self.set_addr_window(x, y, x + w - 1, y + h - 1)?;
        self.tft_spi.write_command(Command::MemoryWrite)?;
        for chunk in buffer.chunks(MAX_BUFFER_SIZE) {
            self.tft_spi.write_data(chunk)?;
        }

        Ok(())
    }

    /// Sets the drawing window (inclusive coordinates).
    pub fn set_addr_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        for (given, max) in [(x0, self.width), (x1, self.width), (y0, self.height), (y1, self.height)]
        {
            if given >= max {
                return Err(Error::Size { given, max });
            }
        }

        let [x0h, x0l] = x0.to_be_bytes();
        let [x1h, x1l] = x1.to_be_bytes();
        let [y0h, y0l] = y0.to_be_bytes();
        let [y1h, y1l] = y1.to_be_bytes();

        self.tft_spi
            .write_reg(Command::ColumnAddressSet, &[x0h, x0l, x1h, x1l])?;
        self.tft_spi
            .write_reg(Command::RowAddressSet, &[y0h, y0l, y1h, y1l])?;

        Ok(())
    }
}
