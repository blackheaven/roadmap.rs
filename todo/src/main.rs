use axum::{
    Json,
    // extract::{Extension, Path},
    routing::{patch, post},
    Router,
    Extension, extract::Path, http::StatusCode,
};
use tokio::signal;
use std::{ sync::{Arc, Mutex}, collections::HashMap};
use serde::{Serialize, Deserialize};

struct AppState {
    id: u8,
    items: HashMap<u8, Item>,
}

#[derive(Serialize, Clone)]
struct Item {
    title: String,
    done: bool,
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(Mutex::new(AppState { id: 0, items: HashMap::new() }));

    // build our application with a route
    let app = Router::new()
        .route("/", post(create).get(list))
        .route("/:id", patch(change).delete(remove))
        .layer(Extension(Arc::clone(&app_state)));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
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

async fn create(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Json(payload): Json<CreatePayload>
) -> Json<CreatedResponse> {
    let mut lstate = state.lock().unwrap();
    let id = lstate.id;
    lstate.id += 1;
    lstate.items.insert(id, Item { title: payload.title, done: false});
    return Json(CreatedResponse { id });
}

#[derive(Deserialize)]
struct CreatePayload {
    title: String,
}

#[derive(Serialize)]
struct CreatedResponse {
    id: u8,
}

async fn list(
    Extension(state): Extension<Arc<Mutex<AppState>>>
) -> Json<Vec<ListItem>> {
    let lstate = state.lock().unwrap();
    return Json(Vec::from_iter(lstate
        .items
        .iter()
        .map(|(id, item)| ListItem {id: *id, title: item.clone().title, done: item.clone().done})));
}

#[derive(Serialize)]
struct ListItem {
    id: u8,
    title: String,
    done: bool,
}

async fn change(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(id): Path<u8>,
    Json(payload): Json<ChangePayload>,
) -> Result<(), StatusCode> {
    let mut lstate = state.lock().unwrap();
    match lstate.items.get_mut(&id) {
        Some(item) => {
            if let Some(new_title) = payload.title {
                item.title = new_title;
            }
            if let Some(new_done) = payload.done {
                item.done = new_done;
            }
            return Ok(());
        },
        None => Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
struct ChangePayload {
    title: Option<String>,
    done: Option<bool>,
}

async fn remove(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
    Path(id): Path<u8>,
) -> Result<(), StatusCode> {
    let mut lstate = state.lock().unwrap();
    match lstate.items.remove(&id) {
        Some(_) => Ok(()),
        None => Err(StatusCode::NOT_FOUND)
    }
}

