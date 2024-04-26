use crate::command_parser::*;
use crate::Frame;
use bytes::Bytes;
use std::time::Duration;

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
    pub key: String,
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
    pub msg: Option<Bytes>,
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
    pub channel: String,
    pub message: Bytes,
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
    pub key: String,
    pub value: Bytes,
    pub expire: Option<Duration>,
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
    pub channels: Vec<String>,
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
    pub channels: Vec<String>,
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
    pub key: String,
    pub value: String,
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
    pub command_name: String,
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
