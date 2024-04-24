use bytes::{Buf, Bytes, BytesMut};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::io;
use std::num::TryFromIntError;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, str, vec};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

type Db = Arc<Vec<RwLock<HashMap<String, Vec<u8>>>>>;
const NUM_SHARDS: u64 = 1024;

fn new_sharded_db() -> Db {
    let mut db = Vec::with_capacity(NUM_SHARDS as usize);
    for _ in 0..NUM_SHARDS {
        db.push(RwLock::new(HashMap::new()));
    }
    Arc::new(db)
}

fn find_db(key: &str) -> usize {
    let mut s = DefaultHasher::new();
    key.hash(&mut s);
    (s.finish() % NUM_SHARDS) as usize
}

#[tokio::main]
async fn main() {
    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db = new_sharded_db();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let db = db.clone();

        tokio::spawn(async move {
            process(db, socket).await;
        });
    }
}

async fn process(db: Db, socket: TcpStream) {
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame.clone()) {
            Ok(Command::Ping(cmd)) => match cmd.msg {
                None => Frame::Simple(FrameSimple::Simple("PONG".to_string())),
                Some(value) => Frame::Array(vec![
                    FrameSimple::Simple("PONG".to_string()),
                    FrameSimple::Bulk(value.clone().into()),
                ]),
            },
            Ok(Command::Set(cmd)) => {
                let mut db = db[find_db(cmd.key())].write().await;
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple(FrameSimple::Simple("OK".to_string()))
            }
            Ok(Command::Get(cmd)) => {
                let db = db[find_db(cmd.key())].read().await;
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Simple(FrameSimple::Bulk(value.clone().into()))
                } else {
                    Frame::Simple(FrameSimple::Null)
                }
            }
            Ok(cmd) => {
                println!("Unimplemented cmd: {:?}", cmd);
                Frame::Simple(FrameSimple::Error("unimplemented".to_string()))
            }
            Err(err) => {
                println!("Unable to parse {:?}: {:?}", frame.clone(), err);
                Frame::Simple(FrameSimple::Error("parse error".to_string()))
            }
        };

        // Respond with an error
        connection.write_frame(&response).await.unwrap();
    }
}

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(1024 * 1024),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        loop {
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    if let Some(frame) = self.parse_frame()? {
                        return Ok(Some(frame));
                    }
                    return Err("connection reset by peer".into());
                }
            }
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Frame>, Error> {
        let mut buf = io::Cursor::new(&self.buffer[..]);
        match Frame::parse(&mut buf) {
            Ok(frame) => {
                let len = buf.position() as usize;
                self.buffer.advance(len);
                Ok(Some(frame))
            }
            Err(FrameParseError::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Array(val) => {
                self.stream.write_u8(b'*').await?;
                write_decimal(&mut self.stream, val.len() as u64).await?;

                for entry in &**val {
                    entry.write(&mut self.stream).await?;
                }
            }
            Frame::Simple(val) => val.write(&mut self.stream).await?,
        }

        self.stream.flush().await
    }
}

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

    async fn write(&self, stream: &mut BufWriter<TcpStream>) -> io::Result<()> {
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

async fn write_decimal(stream: &mut BufWriter<TcpStream>, val: u64) -> io::Result<()> {
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

#[derive(Debug)]
pub enum Command {
    Config(Config),
    Get(Get),
    Publish(Publish),
    Set(Set),
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
    Ping(Ping),
    Unknown(Unknown),
}

impl Command {
    pub fn from_frame(frame: Frame) -> Result<Command, CommandParseError> {
        let mut parse = CommandParser::new(frame)?;
        let command_name = parse.next_string()?.to_lowercase();
        let command = match &command_name[..] {
            "get" => Command::Get(Get::parse_frames(&mut parse)?),
            "publish" => Command::Publish(Publish::parse_frames(&mut parse)?),
            "set" => Command::Set(Set::parse_frames(&mut parse)?),
            "subscribe" => Command::Subscribe(Subscribe::parse_frames(&mut parse)?),
            "unsubscribe" => Command::Unsubscribe(Unsubscribe::parse_frames(&mut parse)?),
            "ping" => Command::Ping(Ping::parse_frames(&mut parse)?),
            "config" => Command::Config(Config::parse_frames(&mut parse)?),
            _ => Command::Unknown(Unknown::new(command_name)),
        };
        parse.finish()?;
        Ok(command)
    }
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

impl Get {
    pub fn new(key: impl ToString) -> Get {
        Get {
            key: key.to_string(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Get, CommandParseError> {
        let key = parse.next_string()?;
        Ok(Get { key })
    }
}

#[derive(Debug, Default)]
pub struct Ping {
    msg: Option<Bytes>,
}

impl Ping {
    pub fn new(msg: Option<Bytes>) -> Ping {
        Ping { msg }
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Ping, CommandParseError> {
        match parse.next_bytes() {
            Ok(msg) => Ok(Ping::new(Some(msg))),
            Err(CommandParseError::EndOfStream) => Ok(Ping::default()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug)]
pub struct Publish {
    channel: String,
    message: Bytes,
}

impl Publish {
    pub fn new(channel: impl ToString, message: Bytes) -> Publish {
        Publish {
            channel: channel.to_string(),
            message,
        }
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Publish, CommandParseError> {
        let channel = parse.next_string()?;
        let message = parse.next_bytes()?;

        Ok(Publish { channel, message })
    }
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: Bytes,
    expire: Option<Duration>,
}

impl Set {
    pub fn new(key: impl ToString, value: Bytes, expire: Option<Duration>) -> Set {
        Set {
            key: key.to_string(),
            value,
            expire,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &Bytes {
        &self.value
    }

    pub fn expire(&self) -> Option<Duration> {
        self.expire
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Set, CommandParseError> {
        let key = parse.next_string()?;
        let value = parse.next_bytes()?;
        let mut expire = None;

        match parse.next_string() {
            Ok(s) if s.to_uppercase() == "EX" => {
                let secs = parse.next_int()?;
                expire = Some(Duration::from_secs(secs));
            }
            Ok(s) if s.to_uppercase() == "PX" => {
                let ms = parse.next_int()?;
                expire = Some(Duration::from_millis(ms));
            }
            Ok(_) => return Err("currently `SET` only supports the expiration option".into()),
            Err(CommandParseError::EndOfStream) => {}
            Err(err) => return Err(err.into()),
        }

        Ok(Set { key, value, expire })
    }
}

#[derive(Debug)]
pub struct Subscribe {
    channels: Vec<String>,
}

impl Subscribe {
    pub fn new(channels: Vec<String>) -> Subscribe {
        Subscribe { channels }
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Subscribe, CommandParseError> {
        let mut channels = vec![parse.next_string()?];

        loop {
            match parse.next_string() {
                Ok(s) => channels.push(s),
                Err(CommandParseError::EndOfStream) => break,
                Err(err) => return Err(err.into()),
            }
        }

        Ok(Subscribe { channels })
    }
}

#[derive(Clone, Debug)]
pub struct Unsubscribe {
    channels: Vec<String>,
}

impl Unsubscribe {
    pub fn new(channels: &[String]) -> Unsubscribe {
        Unsubscribe {
            channels: channels.to_vec(),
        }
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Unsubscribe, CommandParseError> {
        let mut channels = vec![];

        loop {
            match parse.next_string() {
                Ok(s) => channels.push(s),
                Err(CommandParseError::EndOfStream) => break,
                Err(err) => return Err(err),
            }
        }

        Ok(Unsubscribe { channels })
    }
}

#[derive(Debug)]
pub struct Config {
    key: String,
    value: String,
}

impl Config {
    pub fn new(key: impl ToString, value: String) -> Config {
        Config {
            key: key.to_string(),
            value,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &String {
        &self.value
    }

    pub fn parse_frames(parse: &mut CommandParser) -> Result<Config, CommandParseError> {
        let key = parse.next_string()?;
        let value = parse.next_string()?;

        Ok(Config { key, value })
    }
}

#[derive(Debug)]
pub struct Unknown {
    command_name: String,
}

impl Unknown {
    pub fn new(key: impl ToString) -> Unknown {
        Unknown {
            command_name: key.to_string(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.command_name
    }
}

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

// struct BufferedStream {
//     stream: BufWriter<TcpStream>,
//     buffer: BytesMut,
//     // position: u64,
// }
//
// impl BufferedStream {
//     pub fn new(socket: TcpStream) -> Self {
//         Self {
//             stream: BufWriter::new(socket),
//             buffer: BytesMut::with_capacity(1024 * 1024),
//             // position: 0,
//         }
//     }
//
//     // pub fn reset_position(&mut self) {
//     //     self.buffer.set_position(0);
//     //     self.refill();
//     // }
//
//     pub async fn peek_u8(&mut self) -> Result<u8, std::io::Error> {
//         if !self.buffer.has_remaining() {
//             self.refill()?
//         }
//
//         Ok(self.buffer.chunk()[0])
//     }
//
//     pub async fn get_u8(&mut self) -> Result<u8, std::io::Error> {
//         if !self.buffer.has_remaining() {
//             self.refill()?
//         }
//
//         Ok(self.buffer.get_u8())
//     }
//
//     pub async fn skip(&mut self, n: u64) -> Result<(), std::io::Error> {
//         if self.buffer.remaining() < n {
//             self.refill()?
//         }
//
//         self.buffer.advance(n);
//         Ok(())
//     }
//
//     /// Read a new-line terminated decimal
//     pub async fn get_decimal(&mut self) -> Result<u64, std::io::Error> {
//         use atoi::atoi;
//
//         let line = get_line(self.buffer)?;
//
//         atoi::<u64>(line).ok_or_else(|| "protocol error; invalid frame format".into())
//     }
//
//     /// Find a line
//     pub async fn get_line<'a>(&mut self) -> Result<&'a [u8], std::io::Error> {
//         // Scan the bytes directly
//         let start = self.buffer.position() as usize;
//         // Scan to the second to last byte
//         let end = self.buffer.len() - 1;
//
//         for i in start..end {
//             if self.buffer[i] == b'\r' && self.buffer[i + 1] == b'\n' {
//                 // We found a line, update the position to be *after* the \n
//                 self.skip((i + 2) as u64);
//
//                 // Return the line
//                 return Ok(&self.buffer[start..i]);
//             }
//         }
//
//         Err("pouet".into())
//     }
//
//     async fn refill(&mut self) -> Result<(), std::io::Error> {
//         if 0 == self.stream.read_buf(&mut self.buffer).await? {
//             if !self.buffer.is_empty() {
//                 return Err("connection reset by peer".to_string().into());
//             }
//         }
//         return Ok(());
//     }
// }
//
