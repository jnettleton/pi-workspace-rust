use rppal::{
    gpio::{Gpio, InputPin},
    spi::{self, Bus, Mode, SlaveSelect, Spi},
};

/// XPT2046 touch controller on the Waveshare 3.5" RPi LCD (A).
///
/// Uses its own SPI handle on CE1 (`/dev/spidev0.1`, GPIO 7 / physical pin 26).
/// The kernel drives CE0/CE1 per transfer, so the display (CE0) and touch
/// never talk over each other and each keeps its own clock speed.
pub struct TftTouch {
    spi: Spi,
    /// TP_IRQ: low while the panel is touched. GPIO 17 / physical pin 11.
    irq: InputPin,
}

impl TftTouch {
    /// XPT2046 max clock is ~2.5 MHz.
    const SPI_CLOCK_SPEED: u32 = 2_000_000;
    const TP_IRQ: u8 = 17;

    // Control bytes: start bit, channel, 12-bit, differential, PD = 00 (IRQ enabled)
    const READ_X: u8 = 0xD0;
    const READ_Y: u8 = 0x90;

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, Self::SPI_CLOCK_SPEED, Mode::Mode0)?;
        let irq = Gpio::new()?.get(Self::TP_IRQ)?.into_input_pullup();
        Ok(Self { spi, irq })
    }

    pub fn is_touched(&self) -> bool {
        self.irq.is_low()
    }

    /// Raw 12-bit (0..=4095) X/Y readings, or `None` if not touched.
    /// These still need calibrating to screen coordinates.
    pub fn read_raw(&mut self) -> spi::Result<Option<(u16, u16)>> {
        if !self.is_touched() {
            return Ok(None);
        }
        let x = self.read_channel(Self::READ_X)?;
        let y = self.read_channel(Self::READ_Y)?;
        Ok(Some((x, y)))
    }

    fn read_channel(&mut self, cmd: u8) -> spi::Result<u16> {
        let tx = [cmd, 0x00, 0x00];
        let mut rx = [0u8; 3];
        self.spi.transfer(&mut rx, &tx)?;
        Ok((((rx[1] as u16) << 8) | rx[2] as u16) >> 3)
    }
}
