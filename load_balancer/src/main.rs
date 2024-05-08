use reqwest;
use std::sync::Arc;
use tokio::{
    io::{self, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time,
};

struct App {
    next: usize,
    upstreams: Vec<(String, bool)>,
}

impl App {
    pub fn new(upstreams: Vec<String>) -> Self {
        Self {
            next: 0,
            upstreams: upstreams.iter().map(|u| (u.clone(), false)).collect(),
        }
    }

    pub fn upstream(&mut self) -> Option<String> {
        let n = self.upstreams.len();
        for i in 0..n {
            let x = (i + self.next) % n;
            if self.upstreams[x].1 {
                self.next = x;
                return Some(self.upstreams[x].0.clone());
            }
        }

        None
    }

    pub async fn refresh_upstreams(&mut self) {
        println!("Refreshing upstreams");
        for i in 0..self.upstreams.len() {
            let target = String::from("http://") + self.upstreams[i].0.as_str() + "/_healthz";
            let is_up = reqwest::get(target.as_str())
                .await
                .is_ok_and(|response| response.status().is_success());
            println!("* {}: {}", target, is_up);
            self.upstreams[i].1 = is_up;
        }
    }
}

#[tokio::main]
async fn main() {
    let app = Arc::new(Mutex::new(App::new(vec![
        String::from("127.0.0.1:3010"),
        String::from("127.0.0.1:3020"),
    ])));

    let worker_app = app.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let mut app = worker_app.lock().await;
            app.refresh_upstreams().await;
        }
    });

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    loop {
        let app = app.clone();
        let (mut frontend_socket, _) = listener.accept().await.unwrap();
        let mut app = app.lock().await;
        match app.upstream() {
            Some(upstream) => {
                let mut backend_stream = TcpStream::connect(upstream.as_str()).await.unwrap();
                tokio::spawn(async move {
                    let mut frontend_socket = frontend_socket;
                    io::copy_bidirectional(&mut backend_stream, &mut frontend_socket)
                        .await
                        .unwrap();
                });
            }
            None => {
                let _ = frontend_socket
                    .write_all("HTTP/1.1 502 Bad Gateway\r\n".as_bytes())
                    .await;
            }
        }
    }
}
