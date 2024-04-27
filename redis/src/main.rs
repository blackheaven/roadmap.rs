mod buffer;
mod command_parser;
mod error;
use crate::error::*;
mod frame;
use crate::frame::*;
mod command;
use crate::buffer::BufferedStream;
use crate::command::*;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::sync::Arc;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

#[tokio::main]
async fn main() {
    let backend = Arc::new(Backend::new());

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let backend = backend.clone();
        tokio::spawn(async move {
            process_client(&backend, socket).await;
        });
    }
}

async fn process_client(backend: &Arc<Backend>, socket: TcpStream) {
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Frame>();

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame.clone()) {
            Ok(cmd) => {
                backend.process(cmd, &cmd_tx);
                cmd_rx.recv().await.unwrap()
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
    write_stream: BufWriter<OwnedWriteHalf>,
    read_buffer: BufferedStream<OwnedReadHalf>,
}

impl Connection {
    pub fn new(mut socket: TcpStream) -> Self {
        let (read, write) = socket.into_split();
        Self {
            write_stream: BufWriter::new(write),
            read_buffer: BufferedStream::new(read),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, Error> {
        self.read_buffer.reset().await;
        match Frame::parse(&mut self.read_buffer).await {
            Ok(frame) => Ok(Some(frame)),
            Err(FrameParseError::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> io::Result<()> {
        match frame {
            Frame::Array(val) => {
                self.write_stream.write_u8(b'*').await?;
                write_decimal(&mut self.write_stream, val.len() as u64).await?;

                for entry in &**val {
                    entry.write(&mut self.write_stream).await?;
                }
            }
            Frame::Simple(val) => val.write(&mut self.write_stream).await?,
        }

        self.write_stream.flush().await
    }
}

const NUM_SHARDS: u64 = 128;

struct Backend {
    kvs: Vec<KVStore>,
}

impl Backend {
    pub fn new() -> Self {
        let mut kvs = Vec::with_capacity(NUM_SHARDS as usize);
        for _ in 0..NUM_SHARDS {
            let (cmd_tx, mut cmd_rx) =
                mpsc::unbounded_channel::<(KVStoreCommand, mpsc::UnboundedSender<Frame>)>();
            tokio::spawn(async move {
                process_kvstore(&mut cmd_rx).await;
            });

            kvs.push(cmd_tx);
        }

        Self { kvs }
    }

    pub fn process(&self, cmd: Command, respond: &mpsc::UnboundedSender<Frame>) {
        match cmd {
            Command::Ping(cmd) => {
                let response = match cmd.msg {
                    None => Frame::Simple(FrameSimple::Simple("PONG".to_string())),
                    Some(value) => Frame::Array(vec![
                        FrameSimple::Simple("PONG".to_string()),
                        FrameSimple::Bulk(value.clone().into()),
                    ]),
                };
                respond.send(response).unwrap();
            }
            Command::Set(cmd) => {
                self.kvs[Self::select_kvs(cmd.key())]
                    .send((KVStoreCommand::Set(cmd), respond.clone()))
                    .unwrap();
            }
            Command::Get(cmd) => {
                self.kvs[Self::select_kvs(cmd.key())]
                    .send((KVStoreCommand::Get(cmd), respond.clone()))
                    .unwrap();
            }
            _ => {
                println!("Unimplemented cmd: {:?}", cmd);
                let response = Frame::Simple(FrameSimple::Error("unimplemented".to_string()));
                respond.send(response).unwrap();
            }
        };
    }

    fn select_kvs(key: &str) -> usize {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        (s.finish() % NUM_SHARDS) as usize
    }
}

type KVStore = mpsc::UnboundedSender<(KVStoreCommand, mpsc::UnboundedSender<Frame>)>;

enum KVStoreCommand {
    Get(Get),
    Set(Set),
}

async fn process_kvstore(
    cmd_rx: &mut mpsc::UnboundedReceiver<(KVStoreCommand, mpsc::UnboundedSender<Frame>)>,
) {
    let mut db = HashMap::with_capacity(2048);

    while let Some((cmd, respond)) = cmd_rx.recv().await {
        let response = match cmd {
            KVStoreCommand::Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple(FrameSimple::Simple("OK".to_string()))
            }
            KVStoreCommand::Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Simple(FrameSimple::Bulk(value.clone().into()))
                } else {
                    Frame::Simple(FrameSimple::Null)
                }
            }
        };
        respond.send(response).unwrap();
    }
}
