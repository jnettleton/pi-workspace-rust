use std::{error, fmt::Display};

#[derive(Debug)]
pub enum Error {
    /// Error from the SPI/GPIO layer (an `rppal` error on the Pi)
    Spi(Box<dyn error::Error + Send + Sync>),
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

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Spi(err) => Some(err.as_ref()),
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
impl From<rppal::spi::Error> for Error {
    fn from(err: rppal::spi::Error) -> Self {
        Error::Spi(Box::new(err))
    }
}
