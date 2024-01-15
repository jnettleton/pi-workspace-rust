pub enum Command {
    // NoOp = 0x00, // non operation
    SoftReset = 0x01, // soft reset
    // ReadDeviceId = 0x04, // read device id
    // ReadDisplayStatus = 0x09,
    // SleepIn = 0x10, //sleep on
    SleepOut = 0x11, // sleep off
    // PartialModeOn = 0x12, // partial mode
    NormalDisplayModeOn = 0x13, // normal display
    DisplayInversionOff = 0x20, // display invert off
    // DisplayInversionOn = 0x21, // display invert on
    // DisplayOff = 0x28, // display off
    DisplayOn = 0x29, // display on
    // IdleModeOn = 0x39, // idle mode on
    // IdleModeOff = 0x38, // idle mode off
    ColumnAddressSet = 0x2A, // column address set
    RowAddressSet = 0x2B,    //row/page address set
    MemoryWrite = 0x2C,      // memory write
    // MemoryRead = 0x2E, // memory read
    // PartialArea = 0x30, // partial area
    // VerticalScrollingDefinition = 0x33, // vertical scroll def
    InterfacePixelFormat = 0x3A, // interface pixel format
    // MemoryAccessControl = 0x36, // memory access control
    // VerticalScrollingStartAddress = 0x37, //vertical scrolling start address

    // frame rate control
    FrameRateControlNormal = 0xB1,  // normal
    FrameRateControlIdle = 0xB2,    // idle
    FrameRateControlPartial = 0xB3, // partial

    DisplayInversionControl = 0xB4, // display inversion control
    // DisplayFunctionControl = 0xB6, // display function set

    // power control
    PowerControl1 = 0xC0,
    PowerControl2 = 0xC1,
    PowerControl3 = 0xC2,
    PowerControl4 = 0xC3,
    PowerControl5 = 0xC4,

    VcomControl1 = 0xC5, // VCOM control 1

    PositiveGammaControl = 0xE0, // positive gamma correction setting
    NegativeGammaControl = 0xE1, // negative gamma correction setting
}

pub enum TFTMode {
    // Normal,
    // Partial,
    // Idle,
    // Sleep,
    // Invert,
    // DisplayOn,
    DisplayOff,
}

pub enum TFTRotate {
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

pub enum TFTPcbType {
    Red,
    Green,
    Black,
    None,
}
