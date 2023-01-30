extern crate rppal;

use std::time::Duration;
use std::{io, thread};

use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, Polarity, SlaveSelect, Spi};

use super::enums::ST7735Command;

// const TEST: i32 = 1;

// const TFT_MAX_BYTES: i32 = 8; //4 * 1024;
const TFT_RST: u8 = 25;
const TFT_DC: u8 = 24;
const TFT_CS_DISPLAY: u8 = 8;
const TFT_CS_TOUCH: u8 = 7;

pub struct RpiSpi {
    // pub spi_device: Spidev,
    pub spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_rst: OutputPin,
    tft_cs_display: OutputPin, // low active
    tft_cs_touch: OutputPin,   // low active
}

impl RpiSpi {
    pub fn new() -> RpiSpi {
        let spi = Self::create_spi().unwrap();

        let gpio = Gpio::new().unwrap();

        let tft_dc = gpio.get(TFT_DC).unwrap().into_output();
        let tft_rst = gpio.get(TFT_RST).unwrap().into_output();
        let tft_cs_display = gpio.get(TFT_CS_DISPLAY).unwrap().into_output();
        let tft_cs_touch = gpio.get(TFT_CS_TOUCH).unwrap().into_output();

        let mut rpi_spi = RpiSpi {
            spi_device: spi,
            command: true,
            tft_dc,
            tft_rst,
            tft_cs_display,
            tft_cs_touch,
        };

        rpi_spi.dc_set_low().unwrap();
        rpi_spi.tft_cs_touch.set_high();

        rpi_spi
    }

    fn create_spi() -> io::Result<Spi> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 500_000, Mode::Mode0).unwrap();
        spi.set_ss_polarity(Polarity::ActiveLow).unwrap();

        Ok(spi)
    }

    pub fn write_reg(&mut self, cmd: ST7735Command, byte: u8) -> io::Result<()> {
        self.dc_set_low().unwrap();
        self.spi_device.write(&[cmd as u8]).unwrap();

        self.dc_set_high().unwrap();
        self.spi_device.write(&[byte]).unwrap();

        Ok(())
    }

    pub fn write_command_delay(&mut self, cmd: ST7735Command, delay: Duration) -> io::Result<()> {
        self.dc_set_low().unwrap();
        self.spi_device.write(&[cmd as u8]).unwrap();

        if !delay.is_zero() {
            thread::sleep(delay);
        }

        Ok(())
    }

    pub fn write_data_delay(&mut self, data: &[u8], delay: Duration) -> io::Result<()> {
        self.dc_set_high().unwrap();
        self.spi_device.write(data).unwrap();

        if !delay.is_zero() {
            thread::sleep(delay);
        }

        Ok(())
    }

    fn dc_set_low(&mut self) -> io::Result<()> {
        if !self.command {
            self.tft_dc.set_low();
            self.command = true;
        }

        Ok(())
    }

    fn dc_set_high(&mut self) -> io::Result<()> {
        if self.command {
            self.tft_dc.set_high();
            self.command = false;
        }

        Ok(())
    }
}

impl Default for RpiSpi {
    fn default() -> Self {
        Self::new()
    }
}
