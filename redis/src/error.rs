use crate::FrameParseError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Other(String),
    FrameParseError(FrameParseError),
    IO(std::io::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Other(s) => write!(f, "{}", s),
            Error::FrameParseError(s) => write!(f, "{}", s),
            Error::IO(s) => write!(f, "{}", s),
        }
    }
}

impl From<String> for Error {
    fn from(src: String) -> Error {
        Error::Other(src.into())
    }
}

impl From<&str> for Error {
    fn from(src: &str) -> Error {
        src.to_string().into()
    }
}

impl From<FrameParseError> for Error {
    fn from(src: FrameParseError) -> Error {
        Error::FrameParseError(src)
    }
}

impl From<std::io::Error> for Error {
    fn from(src: std::io::Error) -> Error {
        Error::IO(src)
    }
}

impl std::error::Error for Error {}
