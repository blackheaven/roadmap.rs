use axum::{
    http::StatusCode,
    // extract::{Extension, Path},
    routing::get,
    Extension,
    Json,
    Router,
};
use clap;
use serde::Serialize;
use std::sync::Arc;
use tokio::signal;

struct AppState {
    id: String,
}

#[tokio::main]
async fn main() {
    let (port, name) = args();
    let app_state = Arc::new(AppState { id: name });

    // build our application with a route
    let app = Router::new()
        .route("/hello", get(hello))
        .route("/goodbye", get(goodbye))
        .route("/_healthz", get(healthz))
        .layer(Extension(Arc::clone(&app_state)));

    // run it
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("Shutting down..."),
        _ = terminate => println!("Shutting down..."),
    }
}

async fn hello(Extension(state): Extension<Arc<AppState>>) -> Json<Response> {
    return Json(Response {
        message: format!("Hello, I am {}", state.id),
    });
}

async fn goodbye(Extension(state): Extension<Arc<AppState>>) -> Json<Response> {
    return Json(Response {
        message: format!("Goodbye, I was {}", state.id),
    });
}

#[derive(Serialize)]
struct Response {
    message: String,
}

async fn healthz(Extension(_state): Extension<Arc<AppState>>) -> StatusCode {
    return StatusCode::NO_CONTENT;
}

fn args() -> (u16, String) {
    let matches = clap::Command::new("Echo HTTP server")
        .arg(
            clap::Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the port")
                .value_parser(clap::value_parser!(u16))
                .action(clap::ArgAction::Set)
                .required(true),
        )
        .arg(
            clap::Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Sets the name")
                .action(clap::ArgAction::Set)
                .required(true),
        )
        .get_matches();

    // Retrieve values of arguments
    let port: u16 = *matches.get_one("port").unwrap();
    let name: String = matches.get_one::<String>("name").unwrap().clone();

    (port, name)
}
