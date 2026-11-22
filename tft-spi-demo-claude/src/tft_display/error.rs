use std::{error, fmt::Display};

use rppal::spi;

#[derive(Debug)]
pub enum Error {
    Spi(spi::Error),
    Size { given: u16, max: u16 },
    /// Pixel buffer length doesn't match width * height * 3
    ImageSize { given: usize, expected: usize },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spi(err) => write!(f, "SPI Error: {err}"),
            Self::Size { given, max } => write!(f, "Given size: {given}, Max size: {max}"),
            Self::ImageSize { given, expected } => {
                write!(f, "Image buffer is {given} bytes, expected {expected} (width * height * 3)")
            }
        }
    }
}

impl error::Error for Error {}

impl From<spi::Error> for Error {
    fn from(err: spi::Error) -> Self {
        Error::Spi(err)
    }
}
