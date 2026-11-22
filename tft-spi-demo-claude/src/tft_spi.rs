extern crate rppal;

use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};

use rppal::{
    gpio::{Gpio, OutputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

use crate::tft_display::enums::Command;

/// Low-level access to the Waveshare 3.5" RPi LCD (A).
///
/// The board puts a 16-bit shift register in front of the ILI9486, so every
/// SPI write must be a whole number of 16-bit words:
/// - commands and command parameters are sent as `[0x00, byte]`
/// - pixel data (RGB565) is sent as `[hi, lo]` per pixel
pub trait TftSpi {
    fn reset_pin(&mut self);
    /// Sends a command followed by its 8-bit parameters (each padded to 16 bits).
    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> spi::Result<usize>;
    fn write_command(&mut self, cmd: Command) -> spi::Result<usize>;
    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> spi::Result<usize>;
    /// Sends raw bytes with DC high (used for pixel data). Length must be even.
    fn write_data(&mut self, data: &[u8]) -> spi::Result<usize>;
}

#[derive(Clone)]
pub struct TftSpiImpl {
    inner: Arc<Mutex<InnerTftSpi>>,
}

impl TftSpiImpl {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inner: Arc::new(Mutex::new(InnerTftSpi::new()?)),
        })
    }
}

impl TftSpi for TftSpiImpl {
    fn reset_pin(&mut self) {
        self.inner.lock().unwrap().reset_pin()
    }

    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_reg(cmd, params)
    }

    fn write_command(&mut self, cmd: Command) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_command(cmd)
    }

    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> spi::Result<usize> {
        let result = self.write_command(cmd);
        if !delay.is_zero() {
            thread::sleep(delay);
        }
        result
    }

    fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_data(data)
    }
}

struct InnerTftSpi {
    spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_rst: OutputPin,
}

impl InnerTftSpi {
    /// 16 MHz is what the Waveshare `waveshare35a` overlay uses.
    /// Drop this (e.g. to 4 MHz) if you see corruption with long jumper wires.
    const SPI_CLOCK_SPEED: u32 = 16_000_000;

    /// Reset
    /// GPIO 25
    /// Physical Pin 22
    const TFT_RST: u8 = 25;

    /// Command/Data Register Select (LCD_RS)
    /// GPIO 24
    /// Physical Pin 18
    const TFT_DC: u8 = 24;

    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let spi_device = Self::create_spi()?;

        let gpio = Gpio::new()?;
        // Start DC low so it matches `command: true`.
        let tft_dc = gpio.get(Self::TFT_DC)?.into_output_low();
        let tft_rst = gpio.get(Self::TFT_RST)?.into_output_high();

        Ok(Self {
            spi_device,
            command: true,
            tft_dc,
            tft_rst,
        })
    }

    /// Creates SPI interface for display with:
    ///
    /// `Bus::SPI0`
    /// - SPI0 MOSI
    ///   - GPIO 10 / Physical Pin 19
    ///   - LCD display / SPI data input of touch panel
    /// - SPI0 MISO
    ///   - GPIO 9 / Physical Pin 21
    ///   - SPI data output of touch panel
    /// - SPI0 SCLK
    ///   - GPIO 11 / Physical Pin 23
    ///   - SPI clock signal for LCD display / touch panel
    ///
    /// `SlaveSelect::Ss0`
    /// - SPI0 CE0
    ///   - GPIO 8 / Physical Pin 24
    ///   - LCD chip select signal, low level selects LCD
    ///
    /// `Mode::Mode0`
    fn create_spi() -> spi::Result<Spi> {
        Spi::new(Bus::Spi0, SlaveSelect::Ss0, Self::SPI_CLOCK_SPEED, Mode::Mode0)
    }

    fn reset_pin(&mut self) {
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_low();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_high();
        // ILI9486 needs up to 120 ms after reset before accepting commands
        thread::sleep(Duration::from_millis(120));
    }

    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> spi::Result<usize> {
        let mut bytes = self.write_command(cmd)?;
        if !params.is_empty() {
            // Pad each 8-bit parameter to a 16-bit word: [0x00, p]
            let padded: Vec<u8> = params.iter().flat_map(|&p| [0x00, p]).collect();
            bytes += self.write_data(&padded)?;
        }
        Ok(bytes)
    }

    fn write_command(&mut self, cmd: Command) -> spi::Result<usize> {
        self.dc_set_low();
        self.spi_device.write(&[0x00, cmd as u8])
    }

    fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
        debug_assert!(data.len() % 2 == 0, "16-bit bus: data length must be even");
        self.dc_set_high();
        self.spi_device.write(data)
    }

    fn dc_set_low(&mut self) {
        if !self.command {
            self.tft_dc.set_low();
            self.command = true;
        }
    }

    fn dc_set_high(&mut self) {
        if self.command {
            self.tft_dc.set_high();
            self.command = false;
        }
    }
}
