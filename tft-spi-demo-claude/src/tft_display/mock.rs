//! Mock ILI9486 for tests: decodes the column/row address and memory write
//! commands the display sends and paints the pixels into a framebuffer, so
//! tests can check what would be on screen. Test builds only.

// `% 3 == 0` and `chunks_exact` rather than `is_multiple_of` / `as_chunks`
// (Rust 1.87 / 1.88) so the tests also build on older Pi toolchains.
#![allow(clippy::manual_is_multiple_of, clippy::chunks_exact_to_as_chunks)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use super::{enums::Command, tft_display::Result};
use crate::tft_spi::TftSpi;

/// Physical display memory (GRAM) size: the panel's native portrait layout.
pub(crate) const GRAM_W: usize = 320;
pub(crate) const GRAM_H: usize = 480;

/// What the mock panel has received.
///
/// Pixels are recorded twice:
/// - `pixels`: at the logical (column, row) address the display sent, ignoring
///   rotation. Most tests check this, sized to the display's logical size.
/// - `gram`: in physical display memory, after applying MADCTL (0x36) the
///   way the ILI9486 does: MV swaps the column and row addresses, then MX
///   mirrors the column and MY mirrors the row. (For the four MADCTL values
///   the driver uses, the other reading of the datasheet, mirroring before
///   the swap, gives the same result.)
#[derive(Default)]
pub(crate) struct Panel {
    pub(crate) width: usize,
    pub(crate) height: usize,
    /// RGB666 bytes per pixel, as sent, at the logical address
    pub(crate) pixels: Vec<[u8; 3]>,
    pub(crate) cmd: u8,
    pub(crate) params: Vec<u8>,
    /// Address window: x0, x1, y0, y1 (inclusive)
    pub(crate) window: (usize, usize, usize, usize),
    pub(crate) cursor: (usize, usize),
    pub(crate) pixels_written: usize,
    /// Pixels sent after the window was already full
    pub(crate) overflow: usize,
    /// Byte count of each memory write data transfer
    pub(crate) pixel_transfers: Vec<usize>,
    /// Last Memory Access Control (MADCTL) value
    pub(crate) madctl: u8,
    /// Physical display memory, GRAM_W x GRAM_H, row by row
    pub(crate) gram: Vec<[u8; 3]>,
    /// How many times each GRAM pixel was written
    pub(crate) gram_writes: Vec<u32>,
    /// Every command with its parameters, in order (pixel data not included)
    pub(crate) commands: Vec<(u8, Vec<u8>)>,
    /// Delays requested after commands: (command, delay)
    pub(crate) delays: Vec<(u8, Duration)>,
    /// Hardware resets, as the number of commands sent before each one
    pub(crate) resets: Vec<usize>,
}

/// Shared handle so the test can inspect the panel after the display has
/// taken ownership of its `TftSpi`.
#[derive(Clone)]
pub(crate) struct MockSpi(pub(crate) Rc<RefCell<Panel>>);

impl MockSpi {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        MockSpi(Rc::new(RefCell::new(Panel {
            width,
            height,
            pixels: vec![[0; 3]; width * height],
            gram: vec![[0; 3]; GRAM_W * GRAM_H],
            gram_writes: vec![0; GRAM_W * GRAM_H],
            ..Default::default()
        })))
    }
}

/// Where logical address (x, y) lands in GRAM for a MADCTL value, as (column, row).
pub(crate) fn gram_position(madctl: u8, x: usize, y: usize) -> (usize, usize) {
    use super::enums::madctl::{MV, MX, MY};
    let (mut col, mut row) = if madctl & MV != 0 { (y, x) } else { (x, y) };
    assert!(col < GRAM_W && row < GRAM_H, "address ({x}, {y}) is outside GRAM with MADCTL {madctl:#04x}");
    if madctl & MX != 0 {
        col = GRAM_W - 1 - col;
    }
    if madctl & MY != 0 {
        row = GRAM_H - 1 - row;
    }
    (col, row)
}

pub(crate) const CASET: u8 = Command::ColumnAddressSet as u8;
pub(crate) const RASET: u8 = Command::RowAddressSet as u8;
pub(crate) const RAMWR: u8 = Command::MemoryWrite as u8;
pub(crate) const MADCTL: u8 = Command::MemoryAccessControl as u8;

impl TftSpi for MockSpi {
    fn reset_pin(&mut self) {
        let mut p = self.0.borrow_mut();
        let sent = p.commands.len();
        p.resets.push(sent);
    }

    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> Result<usize> {
        let n = self.write_command(cmd)?;
        Ok(n + self.write_data(params)?)
    }

    fn write_command(&mut self, cmd: Command) -> Result<usize> {
        let mut p = self.0.borrow_mut();
        p.cmd = cmd as u8;
        p.params.clear();
        p.commands.push((cmd as u8, Vec::new()));
        if p.cmd == RAMWR {
            p.cursor = (p.window.0, p.window.2);
        }
        Ok(1)
    }

    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> Result<usize> {
        let n = self.write_command(cmd)?;
        self.0.borrow_mut().delays.push((cmd as u8, delay));
        Ok(n)
    }

    fn write_data(&mut self, data: &[u8]) -> Result<usize> {
        let mut p = self.0.borrow_mut();
        let be16 = |b: &[u8]| usize::from(u16::from_be_bytes([b[0], b[1]]));
        if p.cmd != RAMWR {
            if let Some((_, params)) = p.commands.last_mut() {
                params.extend_from_slice(data);
            }
        }
        match p.cmd {
            MADCTL => p.madctl = data[0],
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
                    let (col, row) = gram_position(p.madctl, x, y);
                    p.gram[row * GRAM_W + col] = [px[0], px[1], px[2]];
                    p.gram_writes[row * GRAM_W + col] += 1;
                    p.cursor = if x == p.window.1 { (p.window.0, y + 1) } else { (x + 1, y) };
                }
            }
            _ => {}
        }
        Ok(data.len())
    }
}
