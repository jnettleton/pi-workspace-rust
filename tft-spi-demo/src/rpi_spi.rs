extern crate rppal;

use std::{mem, thread, time::Duration};
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use rppal::{
    gpio::{Gpio, OutputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

use crate::rpi_display::enums::Command;

pub trait RpiSpi {
    fn write_command(&mut self, cmd: Command);
    fn write_command_delay(&mut self, cmd: Command, delay: Duration);
    fn write_data(&mut self, data: &[u8], size: usize);
    fn write_data_delay(&mut self, data: &[u8], size: usize, delay: Duration);
}

#[derive(Clone)]
pub struct RpiSpiImpl {
    inner: Arc<Mutex<InnerRpiSpi>>,
}

impl RpiSpiImpl {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerRpiSpi::new())),
        }
    }
}

impl RpiSpi for RpiSpiImpl {
    fn write_command(&mut self, cmd: Command) {
        self.inner.xlock().write_command(cmd);
    }

    fn write_command_delay(&mut self, cmd: Command, delay: Duration) {
        self.inner.xlock().write_command_delay(cmd, delay);
    }

    fn write_data(&mut self, data: &[u8], size: usize) {
        self.inner.xlock().write_data(data, size);
    }

    fn write_data_delay(&mut self, data: &[u8], delay: Duration) {
        self.inner.xlock().write_data_delay(data, size, delay);
    }
}

struct InnerRpiSpi {
    cmd_buffer: [u8; mem::size_of::<u16>()],
    // pub spi_device: Spidev,
    spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_rst: OutputPin,
    // tft_cs_display: OutputPin, // low active
    // tft_cs_touch: OutputPin,   // low active
}


impl InnerRpiSpi {
    const SPI_CLOCK_SPEED: u32 = 500_000; // 500 kHz

    /// Reset
    /// GPIO 25
    /// Physical Pin 22
    const TFT_RST: u8 = 25;

    /// Command/Data Register Select
    /// GPIO 24
    /// Physical Pin 18
    const TFT_DC: u8 = 24;

    // const TFT_CS_DISPLAY: u8 = 8;
    // const TFT_CS_TOUCH: u8 = 7;

    pub fn new() -> Self {
        let spi = Self::create_spi()?;
        let gpio = Gpio::new()?;

        let tft_dc = gpio.get(Self::TFT_DC).unwrap().into_output();
        let tft_rst = gpio.get(Self::TFT_RST).unwrap().into_output();
        // let tft_cs_display = gpio.get(TFT_CS_DISPLAY).unwrap().into_output();
        // let tft_cs_touch = gpio.get(TFT_CS_TOUCH).unwrap().into_output();

        // rpi_spi.dc_set_low().unwrap();
        // rpi_spi.tft_cs_touch.set_high();

        Self {
            cmd_buffer: [0; mem::size_of::<u16>()],
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
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, Self::SPI_CLOCK_SPEED, Mode::Mode0)?;
        // spi.set_ss_polarity(Polarity::ActiveLow)?; // already the default

        Ok(spi)
    }

    // pub fn write_reg(&mut self, cmd: Command, data: &[u8]) {
    //     self.write_command(cmd)?;
    //
    //     // 16-bit parameters
    //     let data = iter::once(0)
    //         .chain(Itertools::intersperse(data.iter().copied(), 0))
    //         .collect_vec();
    //
    //     let bytes = self.write_data(&data)?;
    // }

    pub fn write_command(&mut self, cmd: Command) {
        self.dc_set_low();

        // self.cmd_buffer[0] = 0;
        self.cmd_buffer[1] = cmd as u8;
        let _ = self.spi_device.write(&self.cmd_buffer);
    }

    pub fn write_command_delay(&mut self, cmd: Command, delay: Duration) {
        self.write_command(cmd);

        if !delay.is_zero() {
            thread::sleep(delay);
        }
    }

    pub fn write_data(&mut self, data: &[u8], size: usize) {
        self.dc_set_high();
        let _ = self.spi_device.write(data);
    }

    pub fn write_data_delay(&mut self, data: &[u8], delay: Duration) {
        self.write_data(data, size);

        if !delay.is_zero() {
            thread::sleep(delay);
        }
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

// impl Default for RpiSpi {
//     fn default() -> Self {
//         Self::new()
//     }
// }
