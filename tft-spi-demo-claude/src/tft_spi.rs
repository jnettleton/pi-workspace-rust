use std::time::Duration;

use crate::tft_display::{enums::Command, tft_display::Result};

#[cfg(target_os = "linux")]
pub use rpi::TftSpiImpl;

/// Low-level access to the ILI9486 on the Waveshare 3.5" RPi LCD (A).
///
/// Callers always speak plain 8-bit ILI9486: one byte per command and
/// parameter (DC low for commands, high for data) and 18-bit pixels, 3 bytes
/// each (see `Color::to_rgb666_bytes`). `TftSpiImpl` (Raspberry Pi, Linux
/// only) translates that for the board's 16-bit interface (see `bus16`);
/// tests use a mock panel that decodes the 8-bit form.
pub trait TftSpi {
    fn reset_pin(&mut self);
    /// Sends a command followed by its parameter bytes.
    fn write_reg(&mut self, cmd: Command, params: &[u8]) -> Result<usize>;
    fn write_command(&mut self, cmd: Command) -> Result<usize>;
    fn write_command_delay(&mut self, cmd: Command, delay: Duration) -> Result<usize>;
    /// Sends raw bytes with DC high (parameters or pixel data).
    fn write_data(&mut self, data: &[u8]) -> Result<usize>;
}

/// How bytes reach the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusWidth {
    /// One byte per command and parameter, 18-bit pixels (as tft-spi-demo does).
    Eight,
    /// 16-bit words, as Waveshare's own driver does (see `bus16`).
    Sixteen,
}

impl BusWidth {
    /// `TFT_BUS=8` or `TFT_BUS=16` (the default).
    pub fn from_env() -> Self {
        match std::env::var("TFT_BUS").as_deref().map(str::trim) {
            Ok("8") => Self::Eight,
            Ok("16") | Err(_) => Self::Sixteen,
            Ok(other) => {
                eprintln!("ignoring TFT_BUS={other:?}: use 8 or 16");
                Self::Sixteen
            }
        }
    }
}

/// The Waveshare 3.5" RPi LCD (A) doesn't take SPI directly: shift registers
/// turn the SPI stream into 16-bit words for the ILI9486's parallel bus,
/// latching one word every 16 clocks while CS is low. So every transfer must
/// be whole 16-bit words:
///
/// - a command byte `c` is sent as `[0x00, c]` (DC low)
/// - each parameter byte `p` as `[0x00, p]` (DC high)
/// - pixels as RGB565, 2 bytes each, big-endian (pixel format `0x55`)
///
/// Single bytes, as tft-spi-demo sends them, get paired with their
/// neighbors, so the panel only sees the right words when the number of
/// bytes before them happens to line up. On the board that showed as bars
/// when transfer sizes changed, landscape fills covering a third of the
/// screen, and unreadable text. With 16-bit words all four rotations fill
/// and text is readable.
pub mod bus16 {
    use crate::tft_display::enums::Command;

    /// Pixel format parameter for 16 bits per pixel (DPI and DBI)
    pub const PIXEL_FORMAT_16: u8 = 0x55;

    pub fn command(cmd: u8) -> [u8; 2] {
        [0x00, cmd]
    }

    /// Data bytes following `last_command`, as the board needs them.
    pub fn data(last_command: u8, data: &[u8]) -> Vec<u8> {
        if last_command == Command::MemoryWrite as u8 {
            pixels(data)
        } else if last_command == Command::InterfacePixelFormat as u8 {
            // The caller asks for 18-bit (0x66); this bus sends 16-bit pixels
            data.iter().flat_map(|_| [0x00, PIXEL_FORMAT_16]).collect()
        } else {
            data.iter().flat_map(|&p| [0x00, p]).collect()
        }
    }

    /// 18-bit pixels (3 bytes each, the top 6 bits used) to RGB565.
    #[allow(clippy::manual_is_multiple_of)] // is_multiple_of needs Rust 1.87+
    pub fn pixels(rgb666: &[u8]) -> Vec<u8> {
        debug_assert!(rgb666.len() % 3 == 0, "{} bytes isn't whole pixels", rgb666.len());
        // `chunks_exact` rather than `as_chunks` (Rust 1.88+) for older Pi toolchains
        #[allow(clippy::chunks_exact_to_as_chunks)]
        rgb666
            .chunks_exact(3)
            .flat_map(|p| {
                let (r, g, b) = (u16::from(p[0]), u16::from(p[1]), u16::from(p[2]));
                (((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)).to_be_bytes()
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::tft_display::color::Color;

        #[test]
        fn commands_and_parameters_are_16_bit_words() {
            assert_eq!(command(0x36), [0x00, 0x36]);
            assert_eq!(data(0x36, &[0x28]), [0x00, 0x28]);
            // CASET 0..479
            assert_eq!(data(0x2A, &[0x00, 0x00, 0x01, 0xDF]), [0, 0x00, 0, 0x00, 0, 0x01, 0, 0xDF]);
            assert!(data(0x2A, &[]).is_empty());
        }

        #[test]
        fn pixel_format_becomes_16_bit() {
            assert_eq!(data(0x3A, &[0x66]), [0x00, 0x55]);
        }

        #[test]
        fn pixels_become_rgb565() {
            let rgb = |c: Color| c.to_rgb666_bytes();
            assert_eq!(pixels(&rgb(Color::RED)), [0xF8, 0x00]);
            assert_eq!(pixels(&rgb(Color::GREEN)), [0x07, 0xE0]);
            assert_eq!(pixels(&rgb(Color::BLUE)), [0x00, 0x1F]);
            assert_eq!(pixels(&rgb(Color::WHITE)), [0xFF, 0xFF]);
            assert_eq!(pixels(&rgb(Color::BLACK)), [0x00, 0x00]);

            let row: Vec<u8> = [Color::RED, Color::BLUE].iter().flat_map(|c| c.to_rgb666_bytes()).collect();
            assert_eq!(data(0x2C, &row), [0xF8, 0x00, 0x00, 0x1F]);
        }

        #[test]
        fn every_transfer_is_whole_words() {
            for (cmd, bytes) in [(0x36, vec![0x28]), (0xE0, vec![0x0F; 15]), (0x2C, vec![0xFC; 3 * 7])] {
                assert_eq!(data(cmd, &bytes).len() % 2, 0, "cmd {cmd:#04x}");
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod rpi {
    use std::sync::{Arc, Mutex};
    use std::{thread, time::Duration};

    use rppal::{
        gpio::{Gpio, OutputPin},
        spi::{self, Bus, Mode, SlaveSelect, Spi},
    };

    use super::{bus16, BusWidth, TftSpi};
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

        /// Runs `f` with the display's chip select released (high), so another
        /// device on SPI0 (the touch controller on CE1) can be used without the
        /// LCD seeing that traffic. Holding the lock also keeps display writes
        /// from interleaving with it.
        pub fn with_display_deselected<R>(&self, f: impl FnOnce() -> R) -> R {
            let mut inner = self.inner.lock().unwrap();
            inner.tft_cs.set_high();
            let result = f();
            inner.tft_cs.set_low();
            result
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
        bus: BusWidth,
        /// Last command sent, so `write_data` knows whether it's pixels
        last_command: u8,
        command: bool,
        tft_dc: OutputPin,
        tft_rst: OutputPin,
        tft_cs: OutputPin,
    }

    impl InnerTftSpi {
        /// Default SPI clock. The Pi rounds down to its core clock divided by
        /// an even number: 16 MHz runs at about 15.6 MHz on a Pi 4, within the
        /// ILI9486's rated write speed. tft-spi-demo's 31.2 MHz (about 27.8
        /// MHz actual) is above that; it hasn't been tried with the 16-bit
        /// bus. Override with the `TFT_SPI_HZ` environment variable.
        const SPI_CLOCK_SPEED: u32 = 16_000_000;

        fn clock_speed() -> u32 {
            match std::env::var("TFT_SPI_HZ") {
                Ok(value) => value.trim().parse().unwrap_or_else(|_| {
                    eprintln!("ignoring TFT_SPI_HZ={value:?}: not a number");
                    Self::SPI_CLOCK_SPEED
                }),
                Err(_) => Self::SPI_CLOCK_SPEED,
            }
        }

        /// Reset
        /// GPIO 25
        /// Physical Pin 22
        const TFT_RST: u8 = 25;

        /// Command/Data Register Select (LCD_RS)
        /// GPIO 24
        /// Physical Pin 18
        const TFT_DC: u8 = 24;

        /// LCD chip select (CE0), driven as a plain GPIO
        /// GPIO 8
        /// Physical Pin 24
        const TFT_CS: u8 = 8;

        fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
            let spi_device = Self::create_spi()?;

            let gpio = Gpio::new()?;
            // Start DC low so it matches `command: true`.
            let mut tft_dc = gpio.get(Self::TFT_DC)?.into_output_low();
            let mut tft_rst = gpio.get(Self::TFT_RST)?.into_output_high();
            // rppal turns pins back into inputs when they're dropped. GPIO 25
            // has a pull-down by default, so RST would go low when the program
            // exits and hold the LCD in reset, blanking whatever was drawn
            // (e.g. after the picture viewer). Keep both pins driven instead.
            tft_dc.set_reset_on_drop(false);
            tft_rst.set_reset_on_drop(false);

            // Hold the LCD's chip select low the whole time, as tft-spi-demo
            // does, instead of letting the SPI hardware raise CE0 after every
            // transfer (CS going high resets the board's 16-bit word counter;
            // with 8-bit transfers that showed bars). Taking GPIO 8 as an
            // output disconnects it from the SPI block; on exit rppal puts it
            // back to CE0.
            let tft_cs = gpio.get(Self::TFT_CS)?.into_output_low();

            let bus = BusWidth::from_env();
            println!("display bus: {bus:?} (set TFT_BUS=8 or 16 to change)");

            Ok(Self {
                spi_device,
                bus,
                last_command: 0,
                command: true,
                tft_dc,
                tft_rst,
                tft_cs,
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
        /// - SPI0 CE0, but GPIO 8 is then taken over as a plain output (see
        ///   `new`), so the SPI block's own chip select isn't used
        ///
        /// `Mode::Mode0`
        fn create_spi() -> spi::Result<Spi> {
            let speed = Self::clock_speed();
            println!("display SPI clock: {speed} Hz requested");
            Spi::new(Bus::Spi0, SlaveSelect::Ss0, speed, Mode::Mode0)
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
            self.last_command = cmd as u8;
            match self.bus {
                BusWidth::Eight => self.spi_device.write(&[cmd as u8]),
                BusWidth::Sixteen => self.spi_device.write(&bus16::command(cmd as u8)),
            }
        }

        fn write_data(&mut self, data: &[u8]) -> spi::Result<usize> {
            self.dc_set_high();
            match self.bus {
                BusWidth::Eight => self.spi_device.write(data),
                BusWidth::Sixteen => self
                    .spi_device
                    .write(&bus16::data(self.last_command, data)),
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
    }
}
