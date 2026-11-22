use crate::tft_display::{
    color::Color,
    enums::{madctl, Command, TFTRotate},
    error::Error,
};
use crate::tft_spi::TftSpi;

use std::{result, time::Duration};

pub type Result<T> = result::Result<T, Error>;

/// spidev's default `bufsiz` is 4096 bytes per transfer. Must be even.
const MAX_BUFFER_SIZE: usize = 4096;

/// Native (portrait) panel size of the ILI9486
const NATIVE_WIDTH: u16 = 320;
const NATIVE_HEIGHT: u16 = 480;

/// Driver for the Waveshare 3.5" RPi LCD (A), ILI9486, 16 bits per pixel.
pub struct TftDisplay<S: TftSpi> {
    tft_spi: S,
    width: u16,
    height: u16,
    rotation: TFTRotate,
}

impl<S: TftSpi> TftDisplay<S> {
    pub fn new(tft_spi: S) -> Self {
        Self {
            tft_spi,
            width: NATIVE_HEIGHT,
            height: NATIVE_WIDTH,
            rotation: TFTRotate::Degrees90,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn rotation(&self) -> TFTRotate {
        self.rotation
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.tft_spi.reset_pin();

        let spi = &mut self.tft_spi;
        spi.write_command_delay(Command::SoftReset, Duration::from_millis(120))?;

        // Interface mode control: defaults
        spi.write_reg(Command::InterfaceModeControl, &[0x00])?;

        spi.write_command_delay(Command::SleepOut, Duration::from_millis(250))?;

        // 16 bits / pixel (RGB565) for both DPI and DBI; the 16-bit shift register
        // on this board can't do 18-bit color.
        spi.write_reg(Command::InterfacePixelFormat, &[0x55])?;

        // Power control 3 / VCOM (values from Waveshare's waveshare35a overlay)
        spi.write_reg(Command::PowerControl3, &[0x44])?;
        spi.write_reg(Command::VcomControl1, &[0x00, 0x00, 0x00, 0x00])?;

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

        self.set_rotation(self.rotation)?;

        let spi = &mut self.tft_spi;
        spi.write_command_delay(Command::NormalDisplayModeOn, Duration::from_millis(10))?;
        spi.write_command_delay(Command::DisplayOn, Duration::from_millis(100))?;

        Ok(())
    }

    /// Sets the orientation. If the image comes out mirrored or upside down,
    /// adjust the MX/MY bits for that case.
    pub fn set_rotation(&mut self, rotation: TFTRotate) -> Result<()> {
        let (value, width, height) = match rotation {
            TFTRotate::Degrees0 => (madctl::MX, NATIVE_WIDTH, NATIVE_HEIGHT),
            TFTRotate::Degrees90 => (madctl::MV, NATIVE_HEIGHT, NATIVE_WIDTH),
            TFTRotate::Degrees180 => (madctl::MY, NATIVE_WIDTH, NATIVE_HEIGHT),
            TFTRotate::Degrees270 => (
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
        let pixel = color.to_be_bytes();
        let buffer: Vec<u8> = pixel.iter().copied().cycle().take(MAX_BUFFER_SIZE).collect();

        let mut remaining = w as usize * h as usize * pixel.len();
        while remaining > 0 {
            let n = remaining.min(MAX_BUFFER_SIZE);
            self.tft_spi.write_data(&buffer[..n])?;
            remaining -= n;
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
