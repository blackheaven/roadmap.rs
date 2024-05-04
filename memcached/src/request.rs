use std::num::{ParseIntError, TryFromIntError};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::{fmt, str};
use tokio::io::AsyncReadExt;

use crate::buffer::BufferedStream;

#[derive(Clone, Debug)]
pub enum Request {
    Get(String),
    Set(String, u16, u64, Vec<u8>, bool),
}

#[derive(Debug)]
pub enum RequestParseError {
    Incomplete,
    Other(String),
}

impl From<String> for RequestParseError {
    fn from(src: String) -> RequestParseError {
        RequestParseError::Other(src.into())
    }
}

impl From<&str> for RequestParseError {
    fn from(src: &str) -> RequestParseError {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for RequestParseError {
    fn from(_src: FromUtf8Error) -> RequestParseError {
        "protocol error; invalid request format".into()
    }
}

impl From<Utf8Error> for RequestParseError {
    fn from(_src: Utf8Error) -> RequestParseError {
        "protocol error; invalid request format".into()
    }
}

impl From<TryFromIntError> for RequestParseError {
    fn from(_src: TryFromIntError) -> RequestParseError {
        "protocol error; invalid request format".into()
    }
}

impl From<ParseIntError> for RequestParseError {
    fn from(_src: ParseIntError) -> RequestParseError {
        "protocol error; invalid request format".into()
    }
}

impl From<std::io::Error> for RequestParseError {
    fn from(_src: std::io::Error) -> RequestParseError {
        RequestParseError::Incomplete
    }
}

impl std::error::Error for RequestParseError {}

impl fmt::Display for RequestParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RequestParseError::Incomplete => "stream ended early".fmt(fmt),
            RequestParseError::Other(s) => s.fmt(fmt),
        }
    }
}

impl Request {
    pub async fn parse<Stream: AsyncReadExt + Unpin>(
        buffer: &mut BufferedStream<Stream>,
    ) -> Result<Request, RequestParseError> {
        let raw_line: Vec<String> = std::str::from_utf8(buffer.get_line().await?.as_ref())?
            .split(' ')
            .map(String::from)
            .collect::<Vec<String>>();
        match raw_line[0].as_str() {
            "set" => {
                let key = &raw_line[1];
                let flags = u16::from_str_radix(raw_line[2].as_str(), 10)?;
                let exptime = u64::from_str_radix(raw_line[3].as_str(), 10)?;
                // let byte_count = usize::from_str_radix(raw_line[4], 10);
                let data = buffer.get_line().await?;
                Ok(Request::Set(
                    key.clone(),
                    flags,
                    exptime,
                    data,
                    raw_line.len() == 6 && raw_line[6].as_str() == "noreply",
                ))
            }
            "get" => Ok(Request::Get(raw_line[1].clone())),
            &_ => Err(RequestParseError::Other(String::from("Unknown command"))),
        }
    }
}
