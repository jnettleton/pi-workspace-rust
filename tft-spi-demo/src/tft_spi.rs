extern crate rppal;

use std::sync::{Arc, Mutex};
use std::{thread, time::Duration};

use rppal::{
    gpio::{Gpio, OutputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

use crate::tft_display::enums::Command;
//use crate::util::MutexExt;
//use dyn_clonable::clonable;

//#[clonable]
pub trait TftSpi: Clone + Sized {
    fn reset_pin(self: &mut Self);
    fn write_reg(self: &mut Self, cmd: Command, data: &[u8]) -> spi::Result<usize>;
    fn write_command(self: &mut Self, cmd: Command) -> spi::Result<usize>;
    fn write_command_delay(self: &mut Self, cmd: Command, delay: Duration) -> spi::Result<usize>;
    fn write_data(self: &mut Self, data: &[u8]) -> spi::Result<usize>;
    fn write_data_delay(self: &mut Self, data: &[u8], delay: Duration) -> spi::Result<usize>;
    fn write_word(self: &mut Self, word: u16) -> spi::Result<usize>;
}

#[derive(Clone)]
pub struct TftSpiImpl {
    inner: Arc<Mutex<InnerTftSpi>>,
}

impl TftSpiImpl {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerTftSpi::new())),
        }
    }
}

impl TftSpi for TftSpiImpl {
    fn reset_pin(&mut self) {
        self.inner.lock().unwrap().reset_pin()
    }

    fn write_reg(&mut self, cmd: Command, data: &[u8]) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_reg(cmd, data)
    }

    fn write_command(&mut self, cmd: Command) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_command(cmd)
    }

    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_command_delay(cmd, delay)
    }

    fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_data(data)
    }

    fn write_data_delay(&mut self, data: &[u8], delay: Duration) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_data_delay(data, delay)
    }

    fn write_word(&mut self, word: u16) -> spi::Result<usize> {
        self.inner.lock().unwrap().write_word(word)
    }
}

struct InnerTftSpi {
    // cmd_buffer: [u8; mem::size_of::<u16>()],
    spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_rst: OutputPin,
    // tft_cs_display: OutputPin, // low active
    // tft_cs_touch: OutputPin,   // low active
}

impl InnerTftSpi {
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
        let spi = Self::create_spi().unwrap();

        let gpio = Gpio::new().unwrap();
        let tft_dc = gpio.get(Self::TFT_DC).unwrap().into_output();
        let tft_rst = gpio.get(Self::TFT_RST).unwrap().into_output();
        // let tft_cs_display = gpio.get(TFT_CS_DISPLAY).unwrap().into_output();
        // let tft_cs_touch = gpio.get(TFT_CS_TOUCH).unwrap().into_output();

        // rpi_spi.dc_set_low().unwrap();
        // rpi_spi.tft_cs_touch.set_high();

        Self {
            // cmd_buffer: [0; mem::size_of::<u16>()],
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
        let spi = Spi::new(
            Bus::Spi0,
            SlaveSelect::Ss0,
            Self::SPI_CLOCK_SPEED,
            Mode::Mode0,
        )?;
        // spi.set_ss_polarity(Polarity::ActiveLow)?; // already the default

        Ok(spi)
    }

    pub fn reset_pin(&mut self) {
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(120));
        self.tft_rst.set_low();
        thread::sleep(Duration::from_millis(120));
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(120));
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

    pub fn write_reg(&mut self, cmd: Command, data: &[u8]) -> spi::Result<usize> {
        let cmd_bytes = self.write_command(cmd)?;
        let data_bytes = self.write_data(data)?;

        Ok(cmd_bytes + data_bytes)
    }

    pub fn write_command(&mut self, cmd: Command) -> spi::Result<usize> {
        self.dc_set_low();
        self.spi_device.write(&[cmd as u8]) // self.cmd_buffer)
    }

    pub fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> spi::Result<usize> {
        let result = self.write_command(cmd);

        if !delay.is_zero() {
            thread::sleep(delay);
        }
        result
    }

    pub fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
        self.dc_set_high();
        self.spi_device.write(data)
    }

    pub fn write_data_delay(&mut self, data: &[u8], delay: Duration) -> spi::Result<usize> {
        let result = self.write_data(data);

        if !delay.is_zero() {
            thread::sleep(delay);
        }
        result
    }

    pub fn write_word(&mut self, value: u16) -> spi::Result<usize> {
        self.write_data(&value.to_be_bytes())
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

// impl Default for TftSpi {
//     fn default() -> Self {
//         Self::new()
//     }
// }
