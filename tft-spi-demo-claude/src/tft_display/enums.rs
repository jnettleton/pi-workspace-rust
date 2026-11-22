/// ILI9486 commands
#[derive(Debug, Clone, Copy)]
pub enum Command {
    // NoOp = 0x00,
    SoftReset = 0x01,
    // SleepIn = 0x10,
    SleepOut = 0x11,
    NormalDisplayModeOn = 0x13,
    // DisplayInversionOff = 0x20,
    // DisplayInversionOn = 0x21,
    // DisplayOff = 0x28,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2A,
    RowAddressSet = 0x2B,
    MemoryWrite = 0x2C,
    MemoryAccessControl = 0x36,
    InterfacePixelFormat = 0x3A,

    InterfaceModeControl = 0xB0,
    FrameRateControlNormal = 0xB1,  // normal, full colors
    FrameRateControlIdle = 0xB2,    // idle, 8 colors
    FrameRateControlPartial = 0xB3, // partial, full colors
    PowerControl3 = 0xC2,
    VcomControl1 = 0xC5,

    PositiveGammaControl = 0xE0,
    NegativeGammaControl = 0xE1,
    DigitalGammaControl1 = 0xE2,
}

/// MADCTL (0x36) bits
pub mod madctl {
    pub const MY: u8 = 0x80; // row address order
    pub const MX: u8 = 0x40; // column address order
    pub const MV: u8 = 0x20; // row/column exchange
    pub const BGR: u8 = 0x08; // panel is wired BGR
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotate {
    /// Portrait, 320 x 480
    Degrees0,
    /// Landscape, 480 x 320
    Degrees90,
    /// Portrait (flipped), 320 x 480
    Degrees180,
    /// Landscape (flipped), 480 x 320
    Degrees270,
}
