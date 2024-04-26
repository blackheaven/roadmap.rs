use bytes::{Buf, Bytes};
use std::convert::TryInto;
use std::io;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;
use std::{fmt, str};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::TcpStream,
};

#[derive(Clone, Debug)]
pub enum Frame {
    Simple(FrameSimple),
    Array(Vec<FrameSimple>),
}

#[derive(Clone, Debug)]
pub enum FrameSimple {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Null,
}

#[derive(Debug)]
pub enum FrameParseError {
    Incomplete,
    Other(String),
}
impl From<String> for FrameParseError {
    fn from(src: String) -> FrameParseError {
        FrameParseError::Other(src.into())
    }
}

impl From<&str> for FrameParseError {
    fn from(src: &str) -> FrameParseError {
        src.to_string().into()
    }
}

impl From<FromUtf8Error> for FrameParseError {
    fn from(_src: FromUtf8Error) -> FrameParseError {
        "protocol error; invalid frame format".into()
    }
}

impl From<TryFromIntError> for FrameParseError {
    fn from(_src: TryFromIntError) -> FrameParseError {
        "protocol error; invalid frame format".into()
    }
}

impl std::error::Error for FrameParseError {}

impl fmt::Display for FrameParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrameParseError::Incomplete => "stream ended early".fmt(fmt),
            FrameParseError::Other(s) => s.fmt(fmt),
        }
    }
}

impl Frame {
    fn push(&mut self, simple: FrameSimple) {
        match self {
            Frame::Array(vec) => {
                vec.push(simple);
            }
            _ => panic!("not an array frame"),
        }
    }

    pub fn parse(src: &mut io::Cursor<&[u8]>) -> Result<Frame, FrameParseError> {
        match get_u8(src)? {
            b'*' => {
                let len = get_decimal(src)?.try_into()?;
                let mut out = Vec::with_capacity(len);

                for _ in 0..len {
                    let t = get_u8(src)?;
                    out.push(FrameSimple::parse(t, src)?);
                }

                Ok(Frame::Array(out))
            }
            simple => FrameSimple::parse(simple, src).map(Frame::Simple),
        }
    }

    pub async fn write(&self, stream: &mut BufWriter<TcpStream>) -> io::Result<()> {
        match self {
            Frame::Array(val) => {
                stream.write_u8(b'*').await?;
                write_decimal(stream, val.len() as u64).await?;

                for entry in &**val {
                    entry.write(stream).await?;
                }
            }
            Frame::Simple(val) => val.write(stream).await?,
        }

        stream.flush().await
    }
}

impl FrameSimple {
    pub fn parse(t: u8, src: &mut io::Cursor<&[u8]>) -> Result<FrameSimple, FrameParseError> {
        match t {
            b'+' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(FrameSimple::Simple(string))
            }
            b'-' => {
                let line = get_line(src)?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(FrameSimple::Error(string))
            }
            b':' => {
                let n = get_decimal(src)?;
                Ok(FrameSimple::Integer(n))
            }
            b'$' => {
                if b'-' == peek_u8(src)? {
                    let line = get_line(src)?;

                    if line != b"-1" {
                        return Err("protocol error; invalid frame format".into());
                    }

                    Ok(FrameSimple::Null)
                } else {
                    // Read the bulk string
                    let len = get_decimal(src)?.try_into()?;
                    let n = len + 2;

                    if src.remaining() < n {
                        return Err(FrameParseError::Incomplete);
                    }

                    let data = Bytes::copy_from_slice(&src.chunk()[..len]);

                    // skip that number of bytes + 2 (\r\n).
                    skip(src, n)?;

                    Ok(FrameSimple::Bulk(data))
                }
            }
            actual => Err(format!("protocol error; invalid frame type byte `{}`", actual).into()),
        }
    }

    pub async fn write(&self, stream: &mut BufWriter<TcpStream>) -> io::Result<()> {
        match self {
            FrameSimple::Simple(val) => {
                stream.write_u8(b'+').await?;
                stream.write_all(val.as_bytes()).await?;
                stream.write_all(b"\r\n").await?;
            }
            FrameSimple::Error(val) => {
                stream.write_u8(b'-').await?;
                stream.write_all(val.as_bytes()).await?;
                stream.write_all(b"\r\n").await?;
            }
            FrameSimple::Integer(val) => {
                stream.write_u8(b':').await?;
                write_decimal(stream, *val).await?;
            }
            FrameSimple::Null => {
                stream.write_all(b"$-1\r\n").await?;
            }
            FrameSimple::Bulk(val) => {
                let len = val.len();

                stream.write_u8(b'$').await?;
                write_decimal(stream, len as u64).await?;
                stream.write_all(val).await?;
                stream.write_all(b"\r\n").await?;
            }
        }

        Ok(())
    }
}

fn peek_u8(src: &mut io::Cursor<&[u8]>) -> Result<u8, FrameParseError> {
    if !src.has_remaining() {
        return Err(FrameParseError::Incomplete);
    }

    Ok(src.chunk()[0])
}

fn get_u8(src: &mut io::Cursor<&[u8]>) -> Result<u8, FrameParseError> {
    if !src.has_remaining() {
        return Err(FrameParseError::Incomplete);
    }

    Ok(src.get_u8())
}

fn skip(src: &mut io::Cursor<&[u8]>, n: usize) -> Result<(), FrameParseError> {
    if src.remaining() < n {
        return Err(FrameParseError::Incomplete);
    }

    src.advance(n);
    Ok(())
}

/// Read a new-line terminated decimal
fn get_decimal(src: &mut io::Cursor<&[u8]>) -> Result<u64, FrameParseError> {
    use atoi::atoi;

    let line = get_line(src)?;

    atoi::<u64>(line).ok_or_else(|| "protocol error; invalid frame format".into())
}

/// Find a line
fn get_line<'a>(src: &mut io::Cursor<&'a [u8]>) -> Result<&'a [u8], FrameParseError> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            // We found a line, update the position to be *after* the \n
            src.set_position((i + 2) as u64);

            // Return the line
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(FrameParseError::Incomplete)
}

pub async fn write_decimal(stream: &mut BufWriter<TcpStream>, val: u64) -> io::Result<()> {
    use std::io::Write;

    // Convert the value to a string
    let mut buf = [0u8; 20];
    let mut buf = io::Cursor::new(&mut buf[..]);
    write!(&mut buf, "{}", val)?;

    let pos = buf.position() as usize;
    stream.write_all(&buf.get_ref()[..pos]).await?;
    stream.write_all(b"\r\n").await?;

    Ok(())
}
