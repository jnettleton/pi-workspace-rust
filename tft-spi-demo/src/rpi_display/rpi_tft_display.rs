use std::{io, time::Duration};

//use gpio::{GpioOut};
//use gpiod::{Chip, Options, Masked, AsValuesMut, Lines, Direction};
//use rppal::system::DeviceInfo;

use super::enums::{ST7735Command, TFTMode, TFTPcbType};
use super::rpi_spi::RpiSpi;

pub struct RpiTftDisplay {
    rpi_spi: RpiSpi,
    mode: TFTMode,
    pcb_type: TFTPcbType,
    // cursor_x: u16,
    // cursor_y: u16,
    tft_width: u16,
    tft_height: u16,
    tft_start_width: u16,
    tft_start_height: u16,
    // tft_buffer: vec!<u8>(),
    // txt_color: u16,
    // txt_bg_color: u16,
    x_start: u16,
    y_start: u16,
}

impl RpiTftDisplay {
    pub fn new() -> Self {
        let rpi_spi = RpiSpi::new();

        // Self { rpi_spi: rpi_spi, _mode: TFTMode::DISPLAYOFF, pcb_type: TFTPcbType::None, outputs: output_lines }
        Self {
            rpi_spi,
            mode: TFTMode::DISPLAYOFF,
            pcb_type: TFTPcbType::None,
            // cursor_x: 0,
            // cursor_y: 0,
            tft_height: 320,
            tft_width: 480,
            tft_start_height: 320,
            tft_start_width: 480,
            // tft_buffer: [],
            // txt_color: 0xFFFF, // white
            // txt_bg_color: 0x0000, // black
            x_start: 0,
            y_start: 0,
        }
    }

    pub fn init_screen_size(
        &mut self,
        x_offset: u16,
        y_offset: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        self.x_start = x_offset;
        self.y_start = y_offset;
        self.tft_width = width;
        self.tft_start_width = width;
        self.tft_height = height;
        self.tft_start_height = height;

        // let bufsize = width * height * 2;
        // let buffer: vec![u8; &bufsize] = [];
        // self.tft_buffer = buffer;

        Ok(())
    }

    pub fn fill_screen(&mut self, color: u16) -> io::Result<()> {
        self.fill_rectangle(0, 0, self.tft_width, self.tft_height, color)?;
        Ok(())
    }

    pub fn fill_rectangle(
        &mut self,
        x: u16,
        y: u16,
        mut w: u16,
        mut h: u16,
        color: u16,
    ) -> io::Result<()> {
        if x >= self.tft_width || y >= self.tft_height {
            return Ok(());
        };
        if (x + w - 1) >= self.tft_height {
            w = self.tft_width - x;
        }
        if (y + h - 1) >= self.tft_height {
            h = self.tft_height - y
        }
        let hi: u8 = (color >> 8) as u8;
        let lo: u8 = color as u8;

        self.set_addr_window(x, y, x + w - 1, y + h - 1)?;
        self.rpi_spi
            .write_command_delay(ST7735Command::RAMWR, Duration::ZERO)?;
        for _ in 0..h {
            for _ in 0..w {
                self.rpi_spi.write_data_delay(&[hi, lo], Duration::ZERO)?;
            }
        }

        Ok(())
    }

    pub fn set_addr_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> io::Result<()> {
        let mut value0 = x0 + self.x_start;
        let mut value1 = x1 + self.x_start;
        self.rpi_spi
            .write_command_delay(ST7735Command::CASET, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(
            &[
                (value0 >> 8) as u8,
                value0 as u8,
                (value1 >> 8) as u8,
                value1 as u8,
            ],
            Duration::ZERO,
        )?;

        value0 = y0 + self.y_start;
        value1 = y1 + self.y_start;
        self.rpi_spi
            .write_command_delay(ST7735Command::RASET, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(
            &[
                (value0 >> 8) as u8,
                value0 as u8,
                (value1 >> 8) as u8,
                value1 as u8,
            ],
            Duration::ZERO,
        )?;

        Ok(())
    }

    pub fn set_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub fn init_pcb_type(&mut self, tft_pcb_type: TFTPcbType) -> io::Result<()> {
        self.pcb_type = tft_pcb_type;
        Ok(())
    }
    pub fn initialize(&mut self) -> io::Result<()> {
        // https://github.com/gavinlyonsrepo/ST7735_TFT_RPI/blob/main/src/ST7735_TFT.cpp
        // https://github.com/maudeve-it/ST7735S-STM32/blob/main/SOURCE/z_displ_ST7735.c

        self.reset_pin();

        self.cmd2_none()?;
        self.pcb_type = TFTPcbType::None;
        Ok(())
    }
    fn _cmd1(&self) -> io::Result<()> {
        Ok(())
    }
    fn cmd2_none(&mut self) -> io::Result<()> {
        self.rpi_spi
            .write_command_delay(ST7735Command::SWRESET, Duration::from_millis(150))?;
        self.rpi_spi
            .write_command_delay(ST7735Command::SLPOUT, Duration::from_millis(500))?;

        self.rpi_spi
            .write_command_delay(ST7735Command::FRMCTR1, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x01, 0x2C, 0x2D], Duration::from_millis(10))?;

        self.rpi_spi
            .write_command_delay(ST7735Command::FRMCTR2, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x01, 0x2C, 0x2D], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::FRMCTR3, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::INVCTR, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(&[0x07], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::PWCTR1, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0xA2, 0x02, 0x84], Duration::from_millis(10))?;

        self.rpi_spi
            .write_command_delay(ST7735Command::PWCTR2, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(&[0xC5], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::PWCTR3, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x0A, 0x00], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::PWCTR4, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x8A, 0x2A], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::PWCTR5, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x8A, 0xEE], Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::VMCTR1, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x0E], Duration::from_millis(10))?;

        self.rpi_spi
            .write_command_delay(ST7735Command::INVOFF, Duration::ZERO)?;

        self.rpi_spi
            .write_command_delay(ST7735Command::COLMOD, Duration::ZERO)?;
        self.rpi_spi
            .write_data_delay(&[0x05], Duration::from_millis(10))?;

        // 480 x 320
        self.rpi_spi
            .write_command_delay(ST7735Command::CASET, Duration::ZERO)?; //0-479
        self.rpi_spi
            .write_data_delay(&[0x00, 0x00, 0x01, 0xDF], Duration::ZERO)?; //0-479

        self.rpi_spi
            .write_command_delay(ST7735Command::RASET, Duration::ZERO)?; //0-319
        self.rpi_spi
            .write_data_delay(&[0x00, 0x00, 0x01, 0x3F], Duration::ZERO)?; //0-319

        self.rpi_spi
            .write_command_delay(ST7735Command::GMCTRP1, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(
            &[
                0x02, 0x1C, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2D, 0x29, 0x25, 0x2B, 0x39, 0x00, 0x01,
                0x03, 0x10,
            ],
            Duration::ZERO,
        )?;

        self.rpi_spi
            .write_command_delay(ST7735Command::GMCTRN1, Duration::ZERO)?;
        self.rpi_spi.write_data_delay(
            &[
                0x3B, 0x1D, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D, 0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00,
                0x02, 0x10,
            ],
            Duration::from_millis(10),
        )?;

        self.rpi_spi
            .write_command_delay(ST7735Command::NORON, Duration::from_millis(10))?;
        self.rpi_spi
            .write_command_delay(ST7735Command::DISPON, Duration::from_millis(100))?;

        Ok(())
    }

    fn _cmd3(&self) -> io::Result<()> {
        Ok(())
    }

    fn reset_pin(&mut self) {
        self.rpi_spi.reset_pin();
    }
}

impl Default for RpiTftDisplay {
    fn default() -> Self {
        Self::new()
    }
}
