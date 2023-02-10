pub mod color;

pub mod error;

use std::{iter, result, thread, time::Duration};

use itertools::Itertools;
use rppal::spi;

use self::{color::Color, error::Error};

use super::{enums::Command, rpi_spi::RpiSpi};

pub type Result<T> = result::Result<T, Error>;

pub struct RpiTftDisplay {
    rpi_spi: RpiSpi,
    // mode: TFTMode,
    // pcb_type: TFTPcbType,
    // cursor_x: u16,
    // cursor_y: u16,
    tft_width: u16,
    tft_height: u16,
    // tft_start_width: u16,
    // tft_start_height: u16,
    // tft_buffer: vec!<u8>(),
    // txt_color: u16,
    // txt_bg_color: u16,
    x_start: u16,
    y_start: u16,
}

impl RpiTftDisplay {
    pub fn new() -> Self {
        let rpi_spi = RpiSpi::new();

        // Self { rpi_spi: rpi_spi, _mode: TFTMode::DisplayOff, pcb_type: TFTPcbType::None, outputs: output_lines }
        Self {
            rpi_spi,
            // mode: TFTMode::DisplayOff,
            // pcb_type: TFTPcbType::None,
            // cursor_x: 0,
            // cursor_y: 0,
            tft_height: 480,
            tft_width: 320,
            // tft_start_height: 320,
            // tft_start_width: 480,
            // tft_buffer: [],
            // txt_color: 0xFFFF, // white
            // txt_bg_color: 0x0000, // black
            x_start: 0,
            y_start: 0,
        }
    }

    // pub fn init_screen_size(
    //     &mut self,
    //     x_offset: u16,
    //     y_offset: u16,
    //     width: u16,
    //     height: u16,
    // ) -> io::Result<()> {
    //     self.x_start = x_offset;
    //     self.y_start = y_offset;
    //     self.tft_width = width;
    //     self.tft_start_width = width;
    //     self.tft_height = height;
    //     self.tft_start_height = height;

    //     // let buf_size = width * height * 2;
    //     // let buffer: vec![u8; &buf_size] = [];
    //     // self.tft_buffer = buffer;

    //     Ok(())
    // }

    pub fn fill_screen(&mut self, color: Color) -> Result<()> {
        self.fill_rectangle(0, 0, self.tft_width, self.tft_height, color)
    }

    pub fn fill_rectangle(
        &mut self,
        x: u16,
        y: u16,
        mut w: u16,
        mut h: u16,
        color: Color,
    ) -> Result<()> {
        if x >= self.tft_width || y >= self.tft_height {
            return Ok(());
        };
        if (x + w - 1) >= self.tft_height {
            w = self.tft_width - x;
        }
        if (y + h - 1) >= self.tft_height {
            h = self.tft_height - y
        }

        let color = [color.red() << 2, color.green() << 2, color.blue() << 2];
        let color_iter = iter::repeat(color)
            .take(h as usize * w as usize)
            .flatten()
            .chunks(4096);

        self.set_addr_window(x, y, x + w - 1, y + h - 1)?;
        self.rpi_spi.write_command(Command::MemoryWrite)?;
        for color in &color_iter {
            self.rpi_spi.write_data(&color.collect_vec())?;
        }

        Ok(())
    }

    pub fn set_addr_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<()> {
        let column_start = x0 + self.x_start;
        let column_end = x1 + self.x_start;

        let row_start = y0 + self.y_start;
        let row_end = y1 + self.y_start;

        if column_start >= self.tft_width {
            return Err(Error::Size {
                given: column_start,
                max: self.tft_width,
            });
        } else if column_end >= self.tft_width {
            return Err(Error::Size {
                given: column_end,
                max: self.tft_width,
            });
        } else if row_start >= self.tft_height {
            return Err(Error::Size {
                given: row_start,
                max: self.tft_height,
            });
        } else if row_end >= self.tft_height {
            return Err(Error::Size {
                given: row_end,
                max: self.tft_height,
            });
        }

        self.rpi_spi.write_reg(
            Command::ColumnAddressSet,
            &[
                (column_start >> 8) as u8,
                column_start as u8,
                (column_end >> 8) as u8,
                column_end as u8,
            ],
        )?;

        self.rpi_spi.write_reg(
            Command::RowAddressSet,
            &[
                (row_start >> 8) as u8,
                row_start as u8,
                (row_end >> 8) as u8,
                row_end as u8,
            ],
        )?;

        Ok(())
    }

    // pub fn set_cursor(&mut self) -> io::Result<()> {
    //     Ok(())
    // }

    // pub fn init_pcb_type(&mut self, tft_pcb_type: TFTPcbType) {
    //     self.pcb_type = tft_pcb_type;
    // }

    pub fn initialize(&mut self) -> spi::Result<()> {
        self.reset_pin();
        self.init_display()?;
        // self.pcb_type = TFTPcbType::None;
        Ok(())
    }

    fn init_display(&mut self) -> spi::Result<()> {
        // Soft reset to set defaults
        self.rpi_spi
            .write_command_delay(Command::SoftReset, Duration::from_millis(150))?;

        // Soft reset sets Sleep In
        self.rpi_spi
            .write_command_delay(Command::SleepOut, Duration::from_millis(500))?;

        let frs = 0b0001; // 30Hz
        let div = 0b00; // fosc
        let rtn = 0b10001; // 17 clocks per line

        // Sets normal mode frame rate to 30Hz, division ratio to fosc, 17 clocks per line
        self.rpi_spi
            .write_reg(Command::FrameRateControlNormal, &[(frs << 4) | div, rtn])?;
        thread::sleep(Duration::from_millis(10));

        // Sets idle mode frame rate, division ratio to fosc, 17 clocks per line
        self.rpi_spi
            .write_reg(Command::FrameRateControlIdle, &[div, rtn])?;

        // Sets partial mode frame rate, division ratio to fosc, 17 clocks per line
        self.rpi_spi
            .write_reg(Command::FrameRateControlPartial, &[div, rtn])?;

        // Software reset sets display inversion off
        // self.rpi_spi.write_command(Command::DisplayInversionOff)?;
        // let zinv = false as u8; // disable Z-inversion
        // let dinv = 0b00; // column inversion
        // self.rpi_spi
        //     .write_reg(Command::DisplayInversionControl, &[(zinv << 4) | dinv])?;

        // Sets positive/negative gamma to +/- 4.4375
        let vrh1 = 0x0E; //  4.4375
        let vrh2 = 0x0E; // -4.4375
        self.rpi_spi
            .write_reg(Command::PowerControl1, &[vrh1, vrh2])?;
        thread::sleep(Duration::from_millis(10));

        // Sets operating voltage step-up factor
        let bt = 0x0; // VGH: Vci1 * 6, VGL: Vci1 * 5
        let vc = 0x0; // External VCI
        self.rpi_spi.write_reg(Command::PowerControl2, &[bt, vc])?;

        let dc0 = 0b011; // 1 H
        let dc1 = 0b011; // 4 H
        let power_ctrl_cmd = [(dc1 << 4) | dc0];

        // Sets operating frequencies of step-up circuit in normal mode
        self.rpi_spi
            .write_reg(Command::PowerControl3, &power_ctrl_cmd)?;

        // Sets operating frequencies of step-up circuit in idle mode
        self.rpi_spi
            .write_reg(Command::PowerControl4, &power_ctrl_cmd)?;

        // Sets operating frequencies of step-up circuit in partial mode
        self.rpi_spi
            .write_reg(Command::PowerControl5, &power_ctrl_cmd)?;

        // self.rpi_spi.write_reg(Command::VcomControl1, &[0x0E])?;
        // thread::sleep(Duration::from_millis(10));

        // Set by software reset
        // // Sets pixel format to 18 bits / pixel
        // let dpi = 0b0110; // 18 bits / pixel
        // let dbi = 0b110; // 18 bits / pixel
        // self.rpi_spi
        //     .write_reg(Command::InterfacePixelFormat, &[(dpi << 4) | dbi])?;
        // thread::sleep(Duration::from_millis(10));

        // 480 x 320
        self.rpi_spi
            .write_reg(Command::ColumnAddressSet, &[0x00, 0x00, 0x01, 0xDF])?; //0-479

        self.rpi_spi
            .write_reg(Command::RowAddressSet, &[0x00, 0x00, 0x01, 0x3F])?; //0-319

        // self.rpi_spi.write_reg(
        //     Command::PositiveGammaControl,
        //     &[
        //         0x02, 0x1C, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2D, 0x29, 0x25, 0x2B, 0x39, 0x00, 0x01,
        //         0x03, 0x10,
        //     ],
        // )?;

        // self.rpi_spi.write_reg(
        //     Command::NegativeGammaControl,
        //     &[
        //         0x3B, 0x1D, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D, 0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00,
        //         0x02, 0x10,
        //     ],
        // )?;
        // thread::sleep(Duration::from_millis(10));

        self.rpi_spi
            .write_command_delay(Command::NormalDisplayModeOn, Duration::from_millis(10))?;
        self.rpi_spi
            .write_command_delay(Command::DisplayOn, Duration::from_millis(100))?;

        Ok(())
    }

    // fn _cmd3(&self) -> io::Result<()> {
    //     Ok(())
    // }

    fn reset_pin(&mut self) {
        self.rpi_spi.reset_pin();
    }
}

impl Default for RpiTftDisplay {
    fn default() -> Self {
        Self::new()
    }
}
