use axum::{
    http::StatusCode,
    // extract::{Extension, Path},
    routing::get,
    Extension,
    Json,
    Router,
};
use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::{signal, time};

struct AppState {
    bucket: usize,
    fw_bucket: usize,
    fw_at: DateTime<Utc>,
    sw: Vec<Instant>,
    sdc: Vec<(Instant, usize)>,
}
const MAX_BUCKET: usize = 12;

#[tokio::main]
async fn main() {
    let app_state = Arc::new(Mutex::new(AppState {
        bucket: 0,
        fw_bucket: MAX_BUCKET,
        fw_at: Utc::now()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap(),
        sw: Vec::with_capacity(20),
        sdc: Vec::with_capacity(5),
    }));
    let worker_state = app_state.clone();
    tokio::spawn(async move { worker(&worker_state).await });

    // build our application with a route
    let app = Router::new()
        .route("/unlimited", get(unlimited))
        .route("/bucket", get(bucket))
        .route("/fixed-window", get(fixed_window))
        .route("/sliding-window", get(sliding_window))
        .route("/sliding-window-count", get(sliding_window_count))
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

#[derive(Serialize)]
struct Response {
    quota: usize,
}

async fn unlimited(Extension(_state): Extension<Arc<Mutex<AppState>>>) -> Json<Response> {
    // let mut lstate = state.lock().unwrap();
    return Json(Response { quota: usize::MAX });
}

async fn bucket(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
) -> Result<Json<Response>, StatusCode> {
    let mut lstate = state.lock().unwrap();
    if lstate.bucket == 0 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    lstate.bucket -= 1;
    return Ok(Json(Response {
        quota: lstate.bucket,
    }));
}

async fn fixed_window(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
) -> Result<Json<Response>, StatusCode> {
    let mut lstate = state.lock().unwrap();
    let now = Utc::now();
    if lstate.fw_at != now {
        lstate.fw_bucket = MAX_BUCKET;
        lstate.fw_at = now;
    }
    if lstate.fw_bucket == 0 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    lstate.fw_bucket -= 1;
    return Ok(Json(Response {
        quota: lstate.fw_bucket,
    }));
}

async fn sliding_window(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
) -> Result<Json<Response>, StatusCode> {
    let mut lstate = state.lock().unwrap();
    lstate
        .sw
        .retain(|&at| at.elapsed() < time::Duration::from_secs(60));
    if lstate.sw.len() == MAX_BUCKET {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    lstate.sw.push(Instant::now());
    return Ok(Json(Response {
        quota: MAX_BUCKET - lstate.sw.len(),
    }));
}

async fn sliding_window_count(
    Extension(state): Extension<Arc<Mutex<AppState>>>,
) -> Result<Json<Response>, StatusCode> {
    let mut lstate = state.lock().unwrap();
    lstate
        .sdc
        .retain(|(at, _)| at.elapsed() < time::Duration::from_secs(60));
    let latest_is_current = lstate.sdc.len() > 0
        && lstate.sdc[lstate.sdc.len() - 1].0.elapsed() < time::Duration::from_secs(30);
    let count: usize = lstate.sdc.iter().map(|(_, count)| *count).sum::<usize>()
        + (if latest_is_current {
            lstate.sdc[lstate.sdc.len() - 1].1
        } else {
            0
        });
    if (count as f64 / 3.0) >= MAX_BUCKET as f64 {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    if latest_is_current {
        let latest = lstate.sdc.len() - 1;
        lstate.sdc[latest].1 += 1;
    } else {
        lstate.sdc.push((Instant::now(), 1));
    }

    return Ok(Json(Response {
        quota: MAX_BUCKET - ((count as f64 + 2.0) / 3.0) as usize,
    }));
}

async fn worker(state: &Arc<Mutex<AppState>>) {
    let mut interval = time::interval(time::Duration::from_secs(5));

    loop {
        interval.tick().await;
        let mut lstate = state.lock().unwrap();
        if lstate.bucket < MAX_BUCKET {
            lstate.bucket += 1;
        }
    }
}
