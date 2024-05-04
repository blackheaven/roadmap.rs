mod buffer;
mod error;
mod request;
mod response;
use crate::buffer::BufferedStream;
use crate::error::*;
use crate::request::*;
use crate::response::*;
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
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Response>();

    let mut connection = Connection::new(socket);

    while let Some(request) = connection.read_request().await.unwrap() {
        backend.process(request, &cmd_tx);
        let response = cmd_rx.recv().await.unwrap();
        connection.write_response(&response).await.unwrap();
    }
}

struct Connection {
    write_stream: BufWriter<OwnedWriteHalf>,
    read_buffer: BufferedStream<OwnedReadHalf>,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        let (read, write) = socket.into_split();
        Self {
            write_stream: BufWriter::new(write),
            read_buffer: BufferedStream::new(read),
        }
    }

    pub async fn read_request(&mut self) -> Result<Option<Request>, Error> {
        self.read_buffer.reset().await;
        match Request::parse(&mut self.read_buffer).await {
            Ok(request) => Ok(Some(request)),
            Err(RequestParseError::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn write_response(&mut self, response: &Response) -> io::Result<()> {
        response.write(&mut self.write_stream).await?;
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
                mpsc::unbounded_channel::<(Request, mpsc::UnboundedSender<Response>)>();
            tokio::spawn(async move {
                process_kvstore(&mut cmd_rx).await;
            });

            kvs.push(cmd_tx);
        }

        Self { kvs }
    }

    pub fn process(&self, request: Request, respond: &mpsc::UnboundedSender<Response>) {
        match request.clone() {
            Request::Set(key, _, _, _, _) => {
                self.kvs[Self::select_kvs(key.clone().as_str())]
                    .send((request.clone(), respond.clone()))
                    .unwrap();
            }
            Request::Get(key) => {
                self.kvs[Self::select_kvs(key.clone().as_str())]
                    .send((request.clone(), respond.clone()))
                    .unwrap();
            }
        };
    }

    fn select_kvs(key: &str) -> usize {
        let mut s = DefaultHasher::new();
        key.hash(&mut s);
        (s.finish() % NUM_SHARDS) as usize
    }
}

type KVStore = mpsc::UnboundedSender<(Request, mpsc::UnboundedSender<Response>)>;

async fn process_kvstore(
    cmd_rx: &mut mpsc::UnboundedReceiver<(Request, mpsc::UnboundedSender<Response>)>,
) {
    let mut db = HashMap::with_capacity(2048);

    while let Some((request, respond)) = cmd_rx.recv().await {
        let response = match request {
            Request::Set(key, flags, _exptime, data, noresponse) => {
                db.insert(key, (data, flags));
                if noresponse {
                    Response::Quiet
                } else {
                    Response::Stored
                }
            }
            Request::Get(key) => {
                if let Some((data, flags)) = db.get(key.as_str()) {
                    Response::Value(key, *flags, data.clone())
                } else {
                    Response::End
                }
            }
        };
        respond.send(response).unwrap();
    }
}
