use std::time::Duration;

use crate::tft_display::{enums::Command, tft_display::Result};

#[cfg(target_os = "linux")]
pub use rpi::TftSpiImpl;

/// Low-level access to the ILI9486 on the Waveshare 3.5" RPi LCD (A).
///
/// 4-wire 8-bit SPI: DC low for a command byte, DC high for parameter and
/// pixel bytes. Over SPI the ILI9486 only supports 18-bit color, so pixels are
/// 3 bytes each (see `Color::to_rgb666_bytes`).
///
/// `TftSpiImpl` is the Raspberry Pi implementation (Linux only); tests use a
/// mock panel instead.
pub trait TftSpi {
    fn reset_pin(&mut self);
    /// Sends a command followed by its parameter bytes.
    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> Result<usize>;
    fn write_command(&mut self, cmd: Command) -> Result<usize>;
    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> Result<usize>;
    /// Sends raw bytes with DC high (parameters or pixel data).
    fn write_data(&mut self, data: &[u8]) -> Result<usize>;
}

#[cfg(target_os = "linux")]
mod rpi {
    use std::sync::{Arc, Mutex};
    use std::{thread, time::Duration};

    use rppal::{
        gpio::{Gpio, OutputPin},
        spi::{self, Bus, Mode, SlaveSelect, Spi},
    };

    use super::TftSpi;
    use crate::tft_display::{enums::Command, tft_display::Result};

    #[derive(Clone)]
    pub struct TftSpiImpl {
        inner: Arc<Mutex<InnerTftSpi>>,
    }

    impl TftSpiImpl {
        pub fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
            Ok(Self {
                inner: Arc::new(Mutex::new(InnerTftSpi::new()?)),
            })
        }
    }

    impl TftSpi for TftSpiImpl {
        fn reset_pin(&mut self) {
            self.inner.lock().unwrap().reset_pin()
        }

        fn write_reg(&mut self, cmd: Command, params: &[u8]) -> Result<usize> {
            Ok(self.inner.lock().unwrap().write_reg(cmd, params)?)
        }

        fn write_command(&mut self, cmd: Command) -> Result<usize> {
            Ok(self.inner.lock().unwrap().write_command(cmd)?)
        }

        fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> Result<usize> {
            let result = self.write_command(cmd);
            if !delay.is_zero() {
                thread::sleep(delay);
            }
            result
        }

        fn write_data(&mut self, data: &[u8]) -> Result<usize> {
            Ok(self.inner.lock().unwrap().write_data(data)?)
        }
    }

    struct InnerTftSpi {
        spi_device: Spi,
        command: bool,
        tft_dc: OutputPin,
        tft_rst: OutputPin,
    }

    impl InnerTftSpi {
        /// Same speed as tft-spi-demo. It's above the ILI9486 datasheet write-cycle
        /// spec, so drop it (e.g. to 16 MHz) if you see corrupted pixels.
        const SPI_CLOCK_SPEED: u32 = 31_200_000;

        /// Reset
        /// GPIO 25
        /// Physical Pin 22
        const TFT_RST: u8 = 25;

        /// Command/Data Register Select (LCD_RS)
        /// GPIO 24
        /// Physical Pin 18
        const TFT_DC: u8 = 24;

        fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
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
            thread::sleep(Duration::from_millis(120));
            self.tft_rst.set_low();
            thread::sleep(Duration::from_millis(120));
            self.tft_rst.set_high();
            // ILI9486 needs up to 120 ms after reset before accepting commands
            thread::sleep(Duration::from_millis(120));
        }

        fn write_reg(&mut self, cmd: Command, params: &[u8]) -> spi::Result<usize> {
            let mut bytes = self.write_command(cmd)?;
            if !params.is_empty() {
                bytes += self.write_data(params)?;
            }
            Ok(bytes)
        }

        fn write_command(&mut self, cmd: Command) -> spi::Result<usize> {
            self.dc_set_low();
            self.spi_device.write(&[cmd as u8])
        }

        fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
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
}
