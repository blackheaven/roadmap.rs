use bytes::Bytes;
use std::convert::TryInto;
use std::io;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;
use std::{fmt, str};
use tokio::io::{AsyncReadExt, AsyncWrite};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::TcpStream,
};

use crate::buffer::BufferedStream;

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

impl From<std::io::Error> for FrameParseError {
    fn from(src: std::io::Error) -> FrameParseError {
        FrameParseError::Incomplete
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

    pub async fn parse<Stream: AsyncReadExt + Unpin>(
        buffer: &mut BufferedStream<Stream>,
    ) -> Result<Frame, FrameParseError> {
        match buffer.get_u8().await? {
            b'*' => {
                let len = buffer.get_decimal().await?.try_into()?;
                let mut out = Vec::with_capacity(len);

                for _ in 0..len {
                    let t = buffer.get_u8().await?;
                    out.push(FrameSimple::parse(t, buffer).await?);
                }

                Ok(Frame::Array(out))
            }
            simple => FrameSimple::parse(simple, buffer).await.map(Frame::Simple),
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
    pub async fn parse<Stream: AsyncReadExt + Unpin>(
        t: u8,
        buffer: &mut BufferedStream<Stream>,
    ) -> Result<FrameSimple, FrameParseError> {
        match t {
            b'+' => {
                let line = buffer.get_line().await?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(FrameSimple::Simple(string))
            }
            b'-' => {
                let line = buffer.get_line().await?.to_vec();
                let string = String::from_utf8(line)?;
                Ok(FrameSimple::Error(string))
            }
            b':' => {
                let n = buffer.get_decimal().await?;
                Ok(FrameSimple::Integer(n))
            }
            b'$' => {
                if b'-' == buffer.peek_u8().await? {
                    let line = buffer.get_line().await?;

                    if line != b"-1" {
                        return Err("protocol error; invalid frame format".into());
                    }

                    Ok(FrameSimple::Null)
                } else {
                    // Read the bulk string
                    let len = buffer.get_decimal().await?.try_into()?;
                    let data = Bytes::copy_from_slice(buffer.take(len).await?.as_slice());

                    // skip that number of bytes + 2 (\r\n).
                    buffer.skip(2).await?;

                    Ok(FrameSimple::Bulk(data))
                }
            }
            actual => Err(format!("protocol error; invalid frame type byte `{}`", actual).into()),
        }
    }

    pub async fn write<T: AsyncWrite + Unpin>(&self, stream: &mut T) -> io::Result<()> {
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

pub async fn write_decimal<Stream: AsyncWrite + Unpin>(
    stream: &mut Stream,
    val: u64,
) -> io::Result<()> {
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
