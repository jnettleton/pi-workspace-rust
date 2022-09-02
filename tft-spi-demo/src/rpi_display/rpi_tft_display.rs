use std::{io, thread};
use std::time::Duration;

//use gpio::{GpioOut};
//use gpiod::{Chip, Options, Masked, AsValuesMut, Lines, Direction};
use rppal::gpio::{Gpio, OutputPin};
//use rppal::system::DeviceInfo;

use super::rpi_spi::RpiSpi;

enum ST7735Command {
    // NOP = 0x00, // non operation
    SWRESET = 0x01, // soft reset
    // RDDID = 0x04, // read device id
    // RDDST = 0x09,
    // SLPIN = 0x10, //sleep on
    SLPOUT = 0x11, // sleep off
    // PTLON = 0x12, // partial mode
    NORON = 0x13, // normal display
    INVOFF = 0x20, // display invert off
    // INVON = 0x21, // display invert on
    // DISPOFF = 0x28, // display off
    DISPON = 0x29, // display on
    // TFT_IDLE_MDODE_ON = 0x39, // idle mode on
    // TFT_IDLE_MODE_OFF = 0x38, // idle mode off
    CASET = 0x2A, // column address set
    RASET = 0x2B, //row/page address set
    RAMWR = 0x2C, // memory write
    // RAMRD = 0x2E, // memory read
    // PTLAR = 0x30, // partial area
    // VSCRDEF = 0x33, // vertical scroll def
    COLMOD = 0x3A, // interface pixel format
    // MADCTL = 0x36, // memory access control
    // VSCRSADD = 0x37, //vertical access control

    // frame rate control
    FRMCTR1 = 0xB1, // normal
    FRMCTR2 = 0xB2, // idle
    FRMCTR3 = 0xB3, // partial

    INVCTR = 0xB4, // display inversion control
    // DISSET5 = 0xB6, // display function set

    // power control
    PWCTR1 = 0xC0,
    PWCTR2 = 0xC1,
    PWCTR3 = 0xC2,
    PWCTR4 = 0xC3,
    PWCTR5 = 0xC4,
    // PWCTR6 = 0xFC,

    VMCTR1 = 0xC5, // VCOM control 1

    // RDDID1 = 0xDA,
    // RDDID2 = 0xDB,
    // RDDID3 = 0xDC,
    // RDDID4 = 0xDD,

    GMCTRP1 = 0xE0, // positive gamma correction setting
    GMCTRN1 = 0xE1, // negative gamma correction setting
}
// enum ST7335MadControl {
//     MADCTL_MY = 0x80,
//     MADCTL_MX = 0x40,
//     MADCTL_MV = 0x20,
//     MADCTL_ML = 0x10,
// }
pub enum ST7735Color {
    BLACK = 0x0000,
    BLUE = 0x001F,
    RED = 0xF800,
    GREEN = 0x07E0,
    CYAN = 0x07FF,
    MAGENTA = 0xF81F,
    YELLOW = 0xFFE0,
    WHITE = 0xFFFF,
    TAN = 0xED01,
    GREY = 0x9CD1,
    BROWN = 0x6201
}
pub enum TFTMode {
    // NORMAL,
    // PARTIAL,
    // IDLE,
    // SLEEP,
    // INVERT,
    // DISPLAYON,
    DISPLAYOFF,
}
pub enum TFTPcbType {
    Red,
    Green,
    Black,
    None
}

pub struct RpiTftDisplay {
    rpi_spi: RpiSpi,
    _mode: TFTMode,
    pcb_type: TFTPcbType,
    // cursor_x: u16,
    // cursor_y: u16,
    tft_width: u16,
    tft_height: u16,
    tft_start_width: u16,
    tft_start_height: u16,
    tft_rst: OutputPin,
    // tft_buffer: vec!<u8>(),
    // txt_color: u16,
    // txt_bg_color: u16,
    x_start: u16,
    y_start: u16,
}

impl RpiTftDisplay {
    pub fn new() -> Self {
        let rpi_spi = RpiSpi::new();
        let gpio25 = Gpio::new().unwrap().get(25).unwrap().into_output();

        // Self { rpi_spi: rpi_spi, _mode: TFTMode::DISPLAYOFF, pcb_type: TFTPcbType::None, outputs: output_lines }
        Self { 
            rpi_spi: rpi_spi,
            _mode: TFTMode::DISPLAYOFF,
            pcb_type: TFTPcbType::None,
            // cursor_x: 0,
            // cursor_y: 0,
            tft_height: 320,
            tft_width: 480,
            tft_start_height: 320,
            tft_start_width: 480,
            tft_rst: gpio25,
            // tft_buffer: [],
            // txt_color: 0xFFFF, // white
            // txt_bg_color: 0x0000, // black
            x_start: 0,
            y_start: 0,
        }
    }

    pub fn init_screen_size(&mut self, x_offset: u16, y_offset: u16, width: u16, height: u16) -> io::Result<()> {
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

    pub fn fill_rectangle(&mut self, x: u16, y: u16, mut w: u16, mut h: u16, color: u16) -> io::Result<()> {
        if x >= self.tft_width || y >= self.tft_height { return Ok(()) };
        if (x + w - 1) >= self.tft_height { w = self.tft_width - x; }
        if (y + h - 1) >= self.tft_height { h = self.tft_height - y }
        let hi: u8 = (color >> 8) as u8;
        let lo: u8 = color as u8;

        self.set_addr_window(x, y, x + w - 1, y + h - 1)?;
        self.rpi_spi.write_command_delay(ST7735Command::RAMWR as u8, 0)?;
        for _i in 0..h {
            for _j in 0..w {
                self.rpi_spi.write_data_delay(&vec![hi, lo], 0)?;
            }
        }

        Ok(())
    }

    pub fn set_addr_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> io::Result<()> {
        let mut value0 = x0 + self.x_start;
        let mut value1 = x1 + self.x_start;
        self.rpi_spi.write_command_delay(ST7735Command::CASET as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![(value0 >> 8) as u8, value0 as u8, (value1 >> 8) as u8, value1 as u8], 0)?;

        value0 = y0 + self.y_start;
        value1 = y1 + self.y_start;
        self.rpi_spi.write_command_delay(ST7735Command::RASET as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![(value0 >> 8) as u8, value0 as u8, (value1 >> 8) as u8, value1 as u8], 0)?;

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
        self.rpi_spi.write_command_delay(ST7735Command::SWRESET as u8, 150)?;
        self.rpi_spi.write_command_delay(ST7735Command::SLPOUT as u8, 500)?;

        self.rpi_spi.write_command_delay(ST7735Command::FRMCTR1 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x01, 0x2C, 0x2D], 10)?;

        self.rpi_spi.write_command_delay(ST7735Command::FRMCTR2 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![ST7735Command::FRMCTR2 as u8, 0x01, 0x2C, 0x2D], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::FRMCTR3 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x01, 0x2C, 0x2D, 0x01, 0x2C, 0x2D], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::INVCTR as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x07], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::PWCTR1 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0xA2, 0x02, 0x84], 10)?;

        self.rpi_spi.write_command_delay(ST7735Command::PWCTR2 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0xC5], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::PWCTR3 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x0A, 0x00], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::PWCTR4 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x8A, 0x2A], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::PWCTR5 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x8A, 0xEE], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::VMCTR1 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x0E], 10)?;

        self.rpi_spi.write_command_delay(ST7735Command::INVOFF as u8, 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::COLMOD as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![0x05], 10)?;

        // 480 x 320
        self.rpi_spi.write_command_delay(ST7735Command::CASET as u8, 0)?; //0-479
        self.rpi_spi.write_data_delay(&vec![0x00, 0x00, 0x01, 0xDF], 0)?; //0-479

        self.rpi_spi.write_command_delay(ST7735Command::RASET as u8, 0)?; //0-319
        self.rpi_spi.write_data_delay(&vec![0x00, 0x00, 0x01, 0x3F], 0)?; //0-319

        self.rpi_spi.write_command_delay(ST7735Command::GMCTRP1 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![
            0x02, 0x1C, 0x07, 0x12, 0x37, 0x32, 0x29, 0x2D,
            0x29, 0x25, 0x2B, 0x39, 0x00, 0x01, 0x03, 0x10], 0)?;

        self.rpi_spi.write_command_delay(ST7735Command::GMCTRN1 as u8, 0)?;
        self.rpi_spi.write_data_delay(&vec![
            0x3B, 0x1D, 0x07, 0x06, 0x2E, 0x2C, 0x29, 0x2D,
            0x2E, 0x2E, 0x37, 0x3F, 0x00, 0x00, 0x02, 0x10], 10)?;

        self.rpi_spi.write_command_delay(ST7735Command::NORON as u8, 10)?;
        self.rpi_spi.write_command_delay(ST7735Command::DISPON as u8, 100)?;

        Ok(())
    }

    fn _cmd3(&self) -> io::Result<()> {
        Ok(())
    }

    fn reset_pin(&mut self) {
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_low();
        thread::sleep(Duration::from_millis(10));
        self.tft_rst.set_high();
        thread::sleep(Duration::from_millis(10));
    }
}

