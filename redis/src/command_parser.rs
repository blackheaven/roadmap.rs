use crate::{Error, Frame, FrameSimple};
use bytes::Bytes;
use std::{fmt, str, vec};

#[derive(Debug)]
pub struct CommandParser {
    parts: vec::IntoIter<FrameSimple>,
}

#[derive(Debug)]
pub enum CommandParseError {
    EndOfStream,
    Other(Error),
}

impl CommandParser {
    pub fn new(frame: Frame) -> Result<CommandParser, CommandParseError> {
        let array = match frame {
            Frame::Array(array) => array,
            frame => return Err(format!("protocol error; expected array, got {:?}", frame).into()),
        };

        Ok(CommandParser {
            parts: array.into_iter(),
        })
    }

    fn next(&mut self) -> Result<FrameSimple, CommandParseError> {
        self.parts.next().ok_or(CommandParseError::EndOfStream)
    }

    pub fn next_string(&mut self) -> Result<String, CommandParseError> {
        match self.next()? {
            FrameSimple::Simple(s) => Ok(s),
            FrameSimple::Bulk(data) => str::from_utf8(&data[..])
                .map(|s| s.to_string())
                .map_err(|_| "protocol error; invalid string".into()),
            frame => Err(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            )
            .into()),
        }
    }

    pub fn next_bytes(&mut self) -> Result<Bytes, CommandParseError> {
        match self.next()? {
            FrameSimple::Simple(s) => Ok(Bytes::from(s.into_bytes())),
            FrameSimple::Bulk(data) => Ok(data),
            frame => Err(format!(
                "protocol error; expected simple frame or bulk frame, got {:?}",
                frame
            )
            .into()),
        }
    }

    pub fn next_int(&mut self) -> Result<u64, CommandParseError> {
        use atoi::atoi;

        const MSG: &str = "protocol error; invalid number";

        match self.next()? {
            FrameSimple::Integer(v) => Ok(v),
            FrameSimple::Simple(data) => atoi::<u64>(data.as_bytes()).ok_or_else(|| MSG.into()),
            FrameSimple::Bulk(data) => atoi::<u64>(&data).ok_or_else(|| MSG.into()),
            frame => Err(format!("protocol error; expected int frame but got {:?}", frame).into()),
        }
    }

    pub fn finish(&mut self) -> Result<(), CommandParseError> {
        if self.parts.next().is_none() {
            Ok(())
        } else {
            Err("protocol error; expected end of frame, but there was more".into())
        }
    }
}

impl From<String> for CommandParseError {
    fn from(src: String) -> CommandParseError {
        CommandParseError::Other(src.into())
    }
}

impl From<&str> for CommandParseError {
    fn from(src: &str) -> CommandParseError {
        src.to_string().into()
    }
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandParseError::EndOfStream => "protocol error; unexpected end of stream".fmt(f),
            CommandParseError::Other(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CommandParseError {}
