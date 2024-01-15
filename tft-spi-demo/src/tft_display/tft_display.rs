use crate::tft_spi::TftSpi;
use crate::tft_display::{
    color::Color,
    enums::{Command, TFTRotate, TFTMode, TFTPcbType},
    error::Error,
};

use std::{result, thread, time::Duration};
// use std::convert::AsMut;

use rppal::spi;

pub type Result<T> = result::Result<T, Error>;

const MAX_BUFFER_SIZE: usize = 4096;

pub struct TftDisplay {
    tft_spi: Box<dyn TftSpi>,
    buffer: [u8; MAX_BUFFER_SIZE],
    mode: TFTMode,
    pcb_type: TFTPcbType,

    x_start: u16,
    y_start: u16,
    // cursor_x: u16,
    // cursor_y: u16,

    tft_width: u16,
    tft_height: u16,
    tft_start_width: u16,
    tft_start_height: u16,
    start_column: u16,
    start_row: u16,
    tft_rotate: TFTRotate,

    // tft_buffer: vec!<u8>(),
    // txt_color: u16,
    // txt_bg_color: u16,
}

const FONT: &[u8; 255] = &[
    0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x5F, 0x00, 0x00,
    0x00, 0x07, 0x00, 0x07, 0x00,
    0x14, 0x7F, 0x14, 0x7F, 0x14,
    0x24, 0x2A, 0x7F, 0x2A, 0x12,
    0x23, 0x13, 0x08, 0x64, 0x62,
    0x36, 0x49, 0x56, 0x20, 0x50,
    0x00, 0x08, 0x07, 0x03, 0x00,
    0x00, 0x1C, 0x22, 0x41, 0x00,
    0x00, 0x41, 0x22, 0x1C, 0x00,
    0x2A, 0x1C, 0x7F, 0x1C, 0x2A,
    0x08, 0x08, 0x3E, 0x08, 0x08,
    0x00, 0x80, 0x70, 0x30, 0x00,
    0x08, 0x08, 0x08, 0x08, 0x08,
    0x00, 0x00, 0x60, 0x60, 0x00,
    0x20, 0x10, 0x08, 0x04, 0x02,
    0x3E, 0x51, 0x49, 0x45, 0x3E,
    0x00, 0x42, 0x7F, 0x40, 0x00,
    0x72, 0x49, 0x49, 0x49, 0x46,
    0x21, 0x41, 0x49, 0x4D, 0x33,
    0x18, 0x14, 0x12, 0x7F, 0x10,
    0x27, 0x45, 0x45, 0x45, 0x39,
    0x3C, 0x4A, 0x49, 0x49, 0x31,
    0x41, 0x21, 0x11, 0x09, 0x07,
    0x36, 0x49, 0x49, 0x49, 0x36,
    0x46, 0x49, 0x49, 0x29, 0x1E,
    0x00, 0x00, 0x14, 0x00, 0x00,
    0x00, 0x40, 0x34, 0x00, 0x00,
    0x00, 0x08, 0x14, 0x22, 0x41,
    0x14, 0x14, 0x14, 0x14, 0x14,
    0x00, 0x41, 0x22, 0x14, 0x08,
    0x02, 0x01, 0x59, 0x09, 0x06,
    0x3E, 0x41, 0x5D, 0x59, 0x4E,
    0x7C, 0x12, 0x11, 0x12, 0x7C,
    0x7F, 0x49, 0x49, 0x49, 0x36,
    0x3E, 0x41, 0x41, 0x41, 0x22,
    0x7F, 0x41, 0x41, 0x41, 0x3E,
    0x7F, 0x49, 0x49, 0x49, 0x41,
    0x7F, 0x09, 0x09, 0x09, 0x01,
    0x3E, 0x41, 0x41, 0x51, 0x73,
    0x7F, 0x08, 0x08, 0x08, 0x7F,
    0x00, 0x41, 0x7F, 0x41, 0x00,
    0x20, 0x40, 0x41, 0x3F, 0x01,
    0x7F, 0x08, 0x14, 0x22, 0x41,
    0x7F, 0x40, 0x40, 0x40, 0x40,
    0x7F, 0x02, 0x1C, 0x02, 0x7F,
    0x7F, 0x04, 0x08, 0x10, 0x7F,
    0x3E, 0x41, 0x41, 0x41, 0x3E,
    0x7F, 0x09, 0x09, 0x09, 0x06,
    0x3E, 0x41, 0x51, 0x21, 0x5E,
    0x7F, 0x09, 0x19, 0x29, 0x46
];
const FONT2: &[u8; 220] = &[
    0x26, 0x49, 0x49, 0x49, 0x32,
    0x03, 0x01, 0x7F, 0x01, 0x03,
    0x3F, 0x40, 0x40, 0x40, 0x3F,
    0x1F, 0x20, 0x40, 0x20, 0x1F,
    0x3F, 0x40, 0x38, 0x40, 0x3F,
    0x63, 0x14, 0x08, 0x14, 0x63,
    0x03, 0x04, 0x78, 0x04, 0x03,
    0x61, 0x59, 0x49, 0x4D, 0x43,
    0x00, 0x7F, 0x41, 0x41, 0x41,
    0x02, 0x04, 0x08, 0x10, 0x20,
    0x00, 0x41, 0x41, 0x41, 0x7F,
    0x04, 0x02, 0x01, 0x02, 0x04,
    0x40, 0x40, 0x40, 0x40, 0x40,
    0x00, 0x03, 0x07, 0x08, 0x00,
    0x20, 0x54, 0x54, 0x78, 0x40,
    0x7F, 0x28, 0x44, 0x44, 0x38,
    0x38, 0x44, 0x44, 0x44, 0x28,
    0x38, 0x44, 0x44, 0x28, 0x7F,
    0x38, 0x54, 0x54, 0x54, 0x18,
    0x00, 0x08, 0x7E, 0x09, 0x02,
    0x18, 0xA4, 0xA4, 0x9C, 0x78,
    0x7F, 0x08, 0x04, 0x04, 0x78,
    0x00, 0x44, 0x7D, 0x40, 0x00,
    0x20, 0x40, 0x40, 0x3D, 0x00,
    0x7F, 0x10, 0x28, 0x44, 0x00,
    0x00, 0x41, 0x7F, 0x40, 0x00,
    0x7C, 0x04, 0x78, 0x04, 0x78,
    0x7C, 0x08, 0x04, 0x04, 0x78,
    0x38, 0x44, 0x44, 0x44, 0x38,
    0xFC, 0x18, 0x24, 0x24, 0x18,
    0x18, 0x24, 0x24, 0x18, 0xFC,
    0x7C, 0x08, 0x04, 0x04, 0x08,
    0x48, 0x54, 0x54, 0x54, 0x24,
    0x04, 0x04, 0x3F, 0x44, 0x24,
    0x3C, 0x40, 0x40, 0x20, 0x7C,
    0x1C, 0x20, 0x40, 0x20, 0x1C,
    0x3C, 0x40, 0x30, 0x40, 0x3C,
    0x44, 0x28, 0x10, 0x28, 0x44,
    0x4C, 0x90, 0x90, 0x90, 0x7C,
    0x44, 0x64, 0x54, 0x4C, 0x44,
    0x00, 0x08, 0x36, 0x41, 0x00,
    0x00, 0x00, 0x77, 0x00, 0x00,
    0x00, 0x41, 0x36, 0x08, 0x00,
    0x02, 0x01, 0x02, 0x04, 0x02
];

impl TftDisplay {
    pub fn new(tft_spi: Box<dyn TftSpi>) -> Self {
        // Self { rpi_spi: rpi_spi, _mode: TFTMode::DisplayOff, pcb_type: TFTPcbType::None, outputs: output_lines }
        Self {
            tft_spi,
            buffer: [0; MAX_BUFFER_SIZE],
            mode: TFTMode::DisplayOff,
            pcb_type: TFTPcbType::None,

            x_start: 0,
            y_start: 0,
            start_column: 0,
            start_row: 0,

            tft_height: 480,
            tft_width: 320,
            tft_start_height: 320,
            tft_start_width: 480,
            tft_rotate: TFTRotate::Degrees0,

            // tft_buffer: [],
            // txt_color: 0xFFFF, // white
            // txt_bg_color: 0x0000, // black
        }
    }

    pub fn init_screen_size(
        &mut self,
        x_offset: u16,
        y_offset: u16,
        width: u16,
        height: u16,
    ) {
        self.x_start = x_offset;
        self.y_start = y_offset;
        self.tft_width = width;
        self.tft_start_width = width;
        self.tft_height = height;
        self.tft_start_height = height;
    }

    pub fn set_rotation(&mut self, rotate: TFTRotate)
    {
        let mut _madctrl: u8 = 0;

        self.tft_rotate = rotate;
        match rotate {
            TFTRotate::Degrees0 => {
                // if self.pcb_type == TFTPcbType::Black {
                //     madctrl = ( ST7735MadControl.MADCTL_MX | ST7735MadControl.MADCTL_MY | ST7735MadControl.MADCTL_RGB) as u8;
                // } else {
                //     // madctrl = (byte) (ST7735MadControl.MADCTL_MX | ST7735MadControl.MADCTL_MY | ST7735MadControl.MADCTL_BGR);
                //     madctrl = (byte) (ST7735MadControl.MADCTL_BGR | ST7735MadControl.MADCTL_MY);
                // }
                // self.tft_width = tft_start_width;
                // self.tft_height = tft_start_height;
                // self.x_start = start_column;
                // self.y_start = start_row;
            }
            TFTRotate::Degrees90 => {
                // if self.pcb_type == TFTPcbType::Black {
                //     madctrl = (byte) (ST7735MadControl.MADCTL_MY | ST7735MadControl.MADCTL_MV | ST7735MadControl.MADCTL_RGB);
                // } else {
                //     // madctrl = (byte) (ST7735MadControl.MADCTL_MY | ST7735MadControl.MADCTL_MV | ST7735MadControl.MADCTL_BGR);
                //     madctrl = (byte) (ST7735MadControl.MADCTL_BGR | ST7735MadControl.MADCTL_MV);
                // }
                // self.tft_width = tft_start_height;
                // self.tft_height = tft_start_width;
                // self.x_start = start_row;
                // self.y_start = start_column;
            }
            TFTRotate::Degrees180 => {
                // if self.pcb_type == TFTPcbType::Black {
                //     madctrl = (byte) (ST7735MadControl.MADCTL_RGB);
                // } else {
                //     // madctrl = (byte) (ST7735MadControl.MADCTL_BGR);
                //     madctrl = (byte) (ST7735MadControl.MADCTL_BGR | ST7735MadControl.MADCTL_MX);
                // }
                // self.tft_width = tft_start_width;
                // self.tft_height = tft_start_height;
                // self.x_start = start_column;
                // self.y_start = start_row;
            }
            TFTRotate::Degrees270 => {
                // if self.pcb_type == TFTPcbType::Black {
                //     madctrl = (byte) (ST7735MadControl.MADCTL_MX | ST7735MadControl.MADCTL_MV | ST7735MadControl.MADCTL_RGB);
                // } else {
                //     // madctrl = (byte) (ST7735MadControl.MADCTL_MX | ST7735MadControl.MADCTL_MV | ST7735MadControl.MADCTL_BGR);
                //     madctrl = (byte) (ST7735MadControl.MADCTL_BGR | ST7735MadControl.MADCTL_MV | ST7735MadControl.MADCTL_MX | ST7735MadControl.MADCTL_MY);
                // }
                // self.tft_width = tft_start_height;
                // self.tft_height = tft_start_width;
                // self.x_start = start_row;
                // self.y_start = start_column;
            }
        }

        // self.rpi_spi.write_command(ST7735Command.MADCTL);
        // self.rpi_spi.write_data(madctrl);
    }

    pub fn fill_screen(&mut self, color: Color) {
        self.fill_rectangle(0, 0, self.tft_width, self.tft_height, color)
    }

    pub fn fill_rectangle(
        &mut self,
        x: u16,
        y: u16,
        mut w: u16,
        mut h: u16,
        color: Color,
    ) {
        if x >= self.tft_width || y >= self.tft_height {
            return;
        };

        let mut width: usize = w.into();
        let mut height: usize = h.into();
        if (x + w - 1) >= self.tft_height {
            width = (self.tft_width - x) as usize;
        }
        if (y + h - 1) >= self.tft_height {
            height = (self.tft_height - y) as usize
        }

        let hi: u8 = color.red() << 2;
        let md: u8 = color.green() << 2;
        let lo = color.blue() << 2;

        let mut index = 0;
        let mut count = width * height;
        while count > 0 {
            if index == MAX_BUFFER_SIZE {
                self.tft_spi.write_data(&self.buffer);
                index = 0;
            }
            self.buffer[index] = hi;
            index = index + 1;

            if index == MAX_BUFFER_SIZE {
                self.tft_spi.write_data(&self.buffer);
                index = 0;
            }
            self.buffer[index] = md;
            index = index + 1;

            if index == MAX_BUFFER_SIZE {
                self.tft_spi.write_data(&self.buffer);
                index = 0;
            }
            self.buffer[index] = lo;
            index = index + 1;

            count = count - 1;
        }
        if index != 0 {
            //let remaining = clone_into_array(&self.buffer[0..index]);
            self.tft_spi.write_data(&self.buffer);
        }

        // let color_iter = iter::repeat(color)
        //     .take(height * width)
        //     .flatten()
        //     .chunks(4096);
        // self.set_addr_window(x, y, x + width - 1, y + height - 1)?;
        // self.rpi_spi.write_command(Command::MemoryWrite)?;
        // for color in &color_iter {
        //     self.rpi_spi.write_data(&color.collect_vec())?;
        // }
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
            })
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

        self.tft_spi.write_reg(
            Command::ColumnAddressSet,
            &[
                (column_start >> 8) as u8,
                column_start as u8,
                (column_end >> 8) as u8,
                column_end as u8,
            ],
        );

        self.tft_spi.write_reg(
            Command::RowAddressSet,
            &[
                (row_start >> 8) as u8,
                row_start as u8,
                (row_end >> 8) as u8,
                row_end as u8,
            ],
        );

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

        self.pcb_type = TFTPcbType::None;
        Ok(())
    }

    fn init_display(&mut self) -> spi::Result<()> {
        // Soft reset to set defaults
        self.tft_spi
            .write_command_delay(Command::SoftReset, Duration::from_millis(150));

        // Soft reset sets Sleep In
        self.tft_spi
            .write_command_delay(Command::SleepOut, Duration::from_millis(500));

        let frs = 0b0001; // 30Hz
        let div = 0b00; // fosc
        let rtn = 0b10001; // 17 clocks per line

        // Sets normal mode frame rate to 30Hz, division ratio to fosc, 17 clocks per line
        self.tft_spi
            .write_reg(Command::FrameRateControlNormal, &[(frs << 4) | div, rtn]);
        thread::sleep(Duration::from_millis(10));

        // Sets idle mode frame rate, division ratio to fosc, 17 clocks per line
        self.tft_spi
            .write_reg(Command::FrameRateControlIdle, &[div, rtn]);

        // Sets partial mode frame rate, division ratio to fosc, 17 clocks per line
        self.tft_spi
            .write_reg(Command::FrameRateControlPartial, &[div, rtn]);

        // Software reset sets display inversion off
        // self.rpi_spi.write_command(Command::DisplayInversionOff)?;
        // let zinv = false as u8; // disable Z-inversion
        // let dinv = 0b00; // column inversion
        // self.rpi_spi
        //     .write_reg(Command::DisplayInversionControl, &[(zinv << 4) | dinv])?;

        // Sets positive/negative gamma to +/- 4.4375
        let vrh1 = 0x0E; //  4.4375
        let vrh2 = 0x0E; // -4.4375
        self.tft_spi
            .write_reg(Command::PowerControl1, &[vrh1, vrh2]);
        thread::sleep(Duration::from_millis(10));

        // Sets operating voltage step-up factor
        let bt = 0x0; // VGH: Vci1 * 6, VGL: Vci1 * 5
        let vc = 0x0; // External VCI
        self.tft_spi.write_reg(Command::PowerControl2, &[bt, vc]);

        let dc0 = 0b011; // 1 H
        let dc1 = 0b011; // 4 H
        let power_ctrl_cmd = [(dc1 << 4) | dc0];

        // Sets operating frequencies of step-up circuit in normal mode
        self.tft_spi
            .write_reg(Command::PowerControl3, &power_ctrl_cmd);

        // Sets operating frequencies of step-up circuit in idle mode
        self.tft_spi
            .write_reg(Command::PowerControl4, &power_ctrl_cmd);

        // Sets operating frequencies of step-up circuit in partial mode
        self.tft_spi
            .write_reg(Command::PowerControl5, &power_ctrl_cmd);

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
        self.tft_spi
            .write_reg(Command::ColumnAddressSet, &[0x00, 0x00, 0x01, 0xDF]); //0-479

        self.tft_spi
            .write_reg(Command::RowAddressSet, &[0x00, 0x00, 0x01, 0x3F]); //0-319

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

        self.tft_spi
            .write_command_delay(Command::NormalDisplayModeOn, Duration::from_millis(10));
        self.tft_spi
            .write_command_delay(Command::DisplayOn, Duration::from_millis(100));

        Ok(())
    }

    // fn _cmd3(&self) -> io::Result<()> {
    //     Ok(())
    // }

    fn reset_pin(&mut self) {
        self.tft_spi.reset_pin();
    }
}

impl Default for TftDisplay {
    fn default() -> Self {
        Self::new()
    }
}
