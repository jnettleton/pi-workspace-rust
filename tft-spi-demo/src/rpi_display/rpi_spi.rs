extern crate rppal;

use std::{io, thread, time};

use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Spi, SlaveSelect, Bus, Mode, Polarity};

#[allow(non_snake_case)]
pub struct RpiSpi {
    // pub spi_device: Spidev,
    pub spi_device: Spi,
    command: bool,
    tft_dc: OutputPin,
    tft_tp_cs: OutputPin, // low active
}

impl RpiSpi {
    pub fn new() -> RpiSpi {
        let spi = Self::create_spi().unwrap();

        let gpio7 = Gpio::new().unwrap().get(7).unwrap().into_output();
        let gpio24 = Gpio::new().unwrap().get(24).unwrap().into_output();
        let mut rpi_spi = RpiSpi {
            spi_device: spi,
            command: true,
            tft_dc: gpio24,
            tft_tp_cs: gpio7,
        };
        rpi_spi.dc_set_low().unwrap();
        rpi_spi.tft_tp_cs.set_high();
        return rpi_spi;
    }

    fn create_spi() -> io::Result<Spi> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 500_000, Mode::Mode0).unwrap();
        spi.set_ss_polarity(Polarity::ActiveLow).unwrap();

        Ok(spi)
    }

    pub fn write_reg(&mut self, cmd: u8, byte: u8) -> io::Result<()> {
        self.dc_set_low().unwrap();
        self.spi_device.write(&[cmd]).unwrap();

        self.dc_set_high().unwrap();
        self.spi_device.write(&[byte]).unwrap();

        Ok(())
    }

    pub fn write_command_delay(&mut self, cmd: u8, delay: u64) -> io::Result<()> {
        self.dc_set_low().unwrap();
        self.spi_device.write(&[cmd]).unwrap();

        if delay != 0 {
            let ms = time::Duration::from_millis(delay);
            thread::sleep(ms);
        }

        Ok(())
    }

    pub fn write_data_delay(&mut self, data: &[u8], delay: u64) -> io::Result<()> {
        self.dc_set_high().unwrap();
        self.spi_device.write(data).unwrap();

        if delay != 0 {
            let ms = time::Duration::from_millis(delay);
            thread::sleep(ms);
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
