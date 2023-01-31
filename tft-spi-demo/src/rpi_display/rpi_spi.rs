extern crate rppal;

use std::{thread, time::Duration};

use rppal::{
    gpio::{Gpio, OutputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

use super::enums::Command;

// const TEST: i32 = 1;

// const TFT_MAX_BYTES: i32 = 8; //4 * 1024;

/// Reset
///
/// GPIO 25
///
/// Physical Pin 22
const TFT_RST: u8 = 25;

/// Command/Data Register Select
///
/// GPIO 24
///
/// Physical Pin 18
const TFT_DC: u8 = 24;

// const TFT_CS_DISPLAY: u8 = 8;
// const TFT_CS_TOUCH: u8 = 7;

pub struct RpiSpi {
    // pub spi_device: Spidev,
    spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_rst: OutputPin,
    // tft_cs_display: OutputPin, // low active
    // tft_cs_touch: OutputPin,   // low active
}

impl RpiSpi {
    pub fn new() -> RpiSpi {
        let spi = Self::create_spi().unwrap();

        let gpio = Gpio::new().unwrap();

        let tft_dc = gpio.get(TFT_DC).unwrap().into_output();
        let tft_rst = gpio.get(TFT_RST).unwrap().into_output();
        // let tft_cs_display = gpio.get(TFT_CS_DISPLAY).unwrap().into_output();
        // let tft_cs_touch = gpio.get(TFT_CS_TOUCH).unwrap().into_output();

        // let mut rpi_spi = RpiSpi {
        //     spi_device: spi,
        //     command: true,
        //     tft_dc,
        //     tft_rst,
        //     tft_cs_display,
        //     tft_cs_touch,
        // };

        // rpi_spi.dc_set_low().unwrap();
        // rpi_spi.tft_cs_touch.set_high();

        // rpi_spi

        RpiSpi {
            spi_device: spi,
            command: true,
            tft_dc,
            tft_rst,
        }
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
    /// `clock_speed: 500_000`
    /// - 500 kHz
    ///
    /// `Mode::Mode0`
    fn create_spi() -> spi::Result<Spi> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 500_000, Mode::Mode0)?;
        // spi.set_ss_polarity(Polarity::ActiveLow)?; // already the default

        Ok(spi)
    }

    pub fn write_reg(&mut self, cmd: Command, byte: u8) -> spi::Result<usize> {
        self.dc_set_low();
        let cmd_bytes = self.spi_device.write(&[cmd as u8])?;

        self.dc_set_high();
        let bytes = self.spi_device.write(&[byte])?;

        Ok(cmd_bytes + bytes)
    }

    pub fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> spi::Result<usize> {
        self.dc_set_low();
        let result = self.spi_device.write(&[cmd as u8]);

        if !delay.is_zero() {
            thread::sleep(delay);
        }

        result
    }

    pub fn write_data_delay(&mut self, data: &[u8], delay: Duration) -> spi::Result<usize> {
        self.dc_set_high();
        let result = self.spi_device.write(data);

        if !delay.is_zero() {
            thread::sleep(delay);
        }

        result
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

    pub fn reset_pin(&mut self) {
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_low();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(10));
    }
}

impl Default for RpiSpi {
    fn default() -> Self {
        Self::new()
    }
}
