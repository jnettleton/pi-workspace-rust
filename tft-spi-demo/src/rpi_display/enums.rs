#[allow(clippy::upper_case_acronyms)]
pub enum Command {
    // NOP = 0x00, // non operation
    SWRESET = 0x01, // soft reset
    // RDDID = 0x04, // read device id
    // RDDST = 0x09,
    // SLPIN = 0x10, //sleep on
    SLPOUT = 0x11, // sleep off
    // PTLON = 0x12, // partial mode
    NORON = 0x13,  // normal display
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
// pub enum ST7735Color {
//     BLACK = 0x0000,
//     BLUE = 0x001F,
//     RED = 0xF800,
//     GREEN = 0x07E0,
//     CYAN = 0x07FF,
//     MAGENTA = 0xF81F,
//     YELLOW = 0xFFE0,
//     WHITE = 0xFFFF,
//     TAN = 0xED01,
//     GREY = 0x9CD1,
//     BROWN = 0x6201,
// }
// pub enum TFTMode {
//     // NORMAL,
//     // PARTIAL,
//     // IDLE,
//     // SLEEP,
//     // INVERT,
//     // DISPLAYON,
//     DISPLAYOFF,
// }
// pub enum TFTPcbType {
//     Red,
//     Green,
//     Black,
//     None,
// }
