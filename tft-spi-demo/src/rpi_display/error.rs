use std::{error, fmt::Display};

use rppal::spi;

#[derive(Debug)]
pub enum Error {
    Spi(spi::Error),
    Size { given: u16, max: u16 },
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spi(err) => write!(f, "SPI Error: {err}"),
            Self::Size { given, max } => write!(f, "Given size: {given}, Max size: {max}"),
        }
    }
}

impl error::Error for Error {}

impl From<spi::Error> for Error {
    fn from(err: spi::Error) -> Self {
        Error::Spi(err)
    }
}
